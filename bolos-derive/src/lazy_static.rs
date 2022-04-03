/*******************************************************************************
*   (c) 2021 Zondax GmbH
*
*  Licensed under the Apache License, Version 2.0 (the "License");
*  you may not use this file except in compliance with the License.
*  You may obtain a copy of the License at
*
*      http://www.apache.org/licenses/LICENSE-2.0
*
*  Unless required by applicable law or agreed to in writing, software
*  distributed under the License is distributed on an "AS IS" BASIS,
*  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
*  See the License for the specific language governing permissions and
*  limitations under the License.
********************************************************************************/
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{
    parse_macro_input, spanned::Spanned, AttributeArgs, Error, Expr, Ident, ItemStatic, Meta,
    NestedMeta, Token, Type,
};

pub fn lazy_static(metadata: TokenStream, input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(metadata as AttributeArgs);
    let input = parse_macro_input!(input as ItemStatic);

    let ItemStatic {
        attrs,
        vis,
        mutability,
        ident: name,
        ty,
        expr,
        ..
    } = input;

    let output = match produce_custom_ty(
        &name,
        *ty,
        *expr,
        mutability.is_some(),
        is_cbindgen_mode(&args),
    )
    .map_err(|e| e.into_compile_error())
    {
        Err(e) => e,
        Ok(CustomTyOut {
            mod_name,
            struct_name,
            body,
        }) => {
            quote! {
                #body

                #(#attrs)*
                #vis static #mutability #name: self::#mod_name::#struct_name = self::#mod_name::#struct_name::new();
            }
        }
    };

    //eprintln!("{}", output);
    output.into()
}

struct CustomTyOut {
    mod_name: Ident,
    struct_name: Ident,
    body: TokenStream2,
}

fn produce_custom_ty(
    name: &Ident,
    ty: Type,
    init: Expr,
    is_mut: bool,
    cbindgen: bool,
) -> Result<CustomTyOut, Error> {
    let span = name.span();
    let mod_name = Ident::new(&format!("__IMPL_LAZY_{}", name), span);
    let struct_name = Ident::new(&format!("__LAZY_{}", name), span);
    let static_name = if cbindgen {
        Ident::new(&format!("{}_LAZY", name), span)
    } else {
        Ident::new("LAZY", span)
    };

    let mut_impl = if is_mut {
        quote! {
            impl #struct_name {
               fn get_mut(&mut self) -> &'static mut #ty {
                   self.init();

                   //SAFETY:
                   // same considerations as `get`:
                   // aligned, non-null, initialized by above call
                   // guaranteed single-threaded access
                   unsafe { #static_name.as_mut_ptr().as_mut().unwrap() }
               }
            }

            impl ::core::ops::DerefMut for #struct_name {
                fn deref_mut(&mut self) -> &mut Self::Target {
                    self.get_mut()
                }
            }
        }
    } else {
        return Err(Error::new(
            is_mut.span(),
            "non-mut static items are not supported!".to_string(),
        ));
    };

    //if we are planning to export this to cbindgen, make the static pub and add #[no_mangle]
    let (cbindgen_attrs, cbindgen_vis): (_, Option<Token![pub]>) = if cbindgen {
        (
            quote!(
                #[no_mangle]
            ),
            Some(Default::default()),
        )
    } else {
        (TokenStream2::new(), None)
    };

    let output = quote! {
        #[allow(non_snake_case)]
        #[doc(hidden)]
        /// This module contains a `lazy_static`-like implementation for ledger devices.
        /// It avoids using atomics as those aren't present in ledger devices, aswell as bypass
        /// the non-standard BSS initialization that is (was) present in the emulator and in the devices,
        /// where BSS was initialized with 0x0A instead of 0x00.
        mod #mod_name {
            use super::*;
            use ::core::mem::MaybeUninit;

            /// Marker, if it is a known value (currently 0x01),
            /// then we have already initialized the statics of this module
            static mut UNINITIALIZED: MaybeUninit<u8> = MaybeUninit::uninit();

            #cbindgen_attrs
            #cbindgen_vis static mut #static_name: MaybeUninit<#ty> = MaybeUninit::uninit();

            #[allow(non_camel_case_types)]
            /// Acts as a "gateway" to the actual value, and is what is exposed outside [#mod_name]
            ///
            /// In the exposed static item, this struct is initialized in a `const` way (via [#struct_name::new])
            /// but as this type implements [::core::ops::Deref] when accessing the static it will deref itself to
            /// [#static_name], which _actually_ contains the _initialized_ data
            pub struct #struct_name {
                __zst: (),
            }

            impl #struct_name {
                pub const fn new() -> Self {
                    Self {
                        __zst: ()
                    }
                }

                /// Called always when attempting to access the inner static
                fn init(&self) {
                    fn __initialize() -> #ty { #init }

                    //SAFETY:
                    // single-threaded code guarantees no data races when accessing
                    // global variables.
                    // Furthermore, u8 can't be uninitialized as any value is valid.
                    let initialized_ptr = unsafe { UNINITIALIZED.as_mut_ptr() };

                    //SAFETY:
                    // ptr comes from rust so guaranteed to be aligned and not null,
                    // is also initialized (see above), not deallocated (global)
                    //
                    // the read is volatile as to avoid the compiler from optimizing out this "simple" value
                    let initialized_val = unsafe { ::core::ptr::read_volatile(initialized_ptr as *const _) };

                    //check against a known value, to avoid the non-standard BSS initialization
                    if initialized_val != 1u8 {
                        //SAFETY:
                        // single threaded access, non-null, aligned
                        unsafe { #static_name.as_mut_ptr().write(__initialize()); };

                        //SAFETY: see above when reading `initialized_val`
                        // write to avoid falling into this branch again!
                        unsafe { initialized_ptr.write(1u8); }
                    }

                }

                fn get(&self) -> &'static #ty {
                    self.init();

                    //SAFETY:
                    // code is single-threaed so no data races,
                    // furthermore the pointer is guaranteed to be non-null, aligned
                    // and initialized by the `init` call above
                    unsafe { #static_name.as_ptr().as_ref().unwrap() }
                }
            }

            impl ::core::ops::Deref for #struct_name {
                type Target = #ty;

                fn deref(&self) -> &Self::Target {
                    self.get()
                }
            }

            #mut_impl
        }
    };

    Ok(CustomTyOut {
        mod_name,
        struct_name,
        body: output,
    })
}

// This function looks for the attribute `cbindgen` in the list of attribute
// args given.
// For example, it will return true for
//#[attr(cbindgen)]
#[allow(clippy::ptr_arg)]
fn is_cbindgen_mode(args: &AttributeArgs) -> bool {
    for arg in args {
        if let NestedMeta::Meta(Meta::Path(path)) = arg {
            if path
                .segments
                .iter()
                .any(|path_segment| path_segment.ident == "cbindgen")
            {
                return true;
            }
        }
    }

    false
}
