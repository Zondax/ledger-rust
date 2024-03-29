/*******************************************************************************
*   (c) 2022 Zondax AG
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

//! This crate exports 4 macros that are useful if not essential for correct
//! and ergonomic rust in a ledger app
//!
//! The currently exported macros are:
//! * [macro@nvm]
//! * [macro@pic]
//! * [macro@pic_str]
//! * [macro@lazy_static]
//! * [macro@enum_init]

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemStatic};

use proc_macro_error::proc_macro_error;

pub(crate) mod utils;

// #[bolos::nvm]
// static mut __FLASH: [[u8; 0xFFFF]; ..];
//
// static mut __FLASH: PIC<NVM<0xFFFF * ..>> = PIC::new(NVM::zeroed());
mod nvm;
#[proc_macro_attribute]
/// This attribute macro is to be applied on top of static items to make sure
/// the item will end up in non-volatile memory (NVM), it will also wrap the item in
/// appropricate types so runtime usage it correct aswell.
///
/// # What's possible
/// ## Initialization
/// NVM storage can be initialized by default with zeros or specifying the initialization array.
/// ```rust
/// #[bolos_derive::nvm]
/// static mut FOO: [u8; 42]; //initialized with all zeroes
///
/// # assert_eq!(unsafe { FOO[30] }, 0);
/// #[bolos_derive::nvm]
/// static mut ONES_AND_TWOS: [u8; 2] = [1, 2];
///
/// assert_eq!(unsafe { ONES_AND_TWOS[0] }, 1);
/// assert_eq!(unsafe { ONES_AND_TWOS[1] }, 2);
/// ```
///
/// ## Multi dimensional arrays
/// It's possible to declare a multi dimensional array, but it will be flattened to one-dimensional.
/// Initialization follows the same rules as above,
/// where the init subarray must be the first dimension initialization array.
///
/// ```rust
/// #[bolos_derive::nvm]
/// static mut FOO: [[u8; 10]; 20] = [33; 10]; //initialize a 10 * 20 array with all 33
///
/// assert_eq!(unsafe { **FOO }, [33u8; 10*20]);
/// ```
pub fn nvm(metadata: TokenStream, input: TokenStream) -> TokenStream {
    nvm::nvm(metadata, input)
}

// #[bolos::pic]
// static mut BUFFER: MyBuffer = MyBuffer::new();
//
// static mut BUFFER: PIC<MyBuffer> = PIC::new(MyBuffer::new());

#[proc_macro_attribute]
pub fn pic(_: TokenStream, input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as ItemStatic);

    let ItemStatic {
        attrs,
        ident: name,
        mutability,
        ty,
        vis,
        expr,
        ..
    } = input;
    let ty = *ty;
    let expr = *expr;

    let output = quote! {
        #(#attrs)*
        #vis static #mutability #name: ::bolos::PIC<#ty> = ::bolos::PIC::new(#expr);
    };

    output.into()
}

mod pic_str;
#[proc_macro]
/// This macro is to be used when a str literal is needed.
/// The macro will automatically use `PIC` to guarantee proper access at runtime
/// as well as null terminate the string (if not already).
///
/// It's possible to avoid null termination by appending a `!` at the end of the string
pub fn pic_str(input: TokenStream) -> TokenStream {
    pic_str::pic_str(input)
}

// #[bolos::static]
// static mut LAZY_STATIC_OBJECT: Object = Object::new()
mod lazy_static;

#[proc_macro_attribute]
pub fn lazy_static(metadata: TokenStream, input: TokenStream) -> TokenStream {
    lazy_static::lazy_static(metadata, input)
}

mod enum_init;

#[proc_macro_error]
#[proc_macro_attribute]
/// The aim of this macro is to ease the writing of boilerplate for enums
/// where we want to initialize said enum using [`MaybeUninit`].
///
/// The macro will generate an enum with N unit variants and N structs
/// based on the number of variants of the original enum.
///
/// # Example
/// ```rust,ignore
/// #[enum_init]
/// pub enum Foo<'b> {
///     Bar(BarStruct<'b>),
///     #[cfg(feature = "baz")]
///     Baz {
///         baz: u8
///         bazz: &'b [u8]
///     }
/// }
///
/// //will generate ***-----------------------------****
///
/// #[repr(u8)]
/// enum Foo__Type {
///     Bar,
///     #[cfg(feature = "baz")]
///     Baz
/// }
///
/// #[repr(C)]
/// #[cfg(feature = "baz")]
/// struct Baz<'b> {
///     baz: u8
///     bazz: &'b [u8]
/// }
///
/// #[repr(C)]
/// struct Bar__Variant<'b>(FooType, BarStruct<'b>);
///
/// #[repr(C)]
/// #[cfg(feature = "baz")]
/// struct Baz__Variant<'b>(FooType, Baz<'b>);
///
/// impl Foo<'b> {
///     pub fn init_as_bar<__T, __F>(mut init: __F, out: &mut MaybeUninit<Self>) -> __T
///     where
///         __F: FnMut(&mut MaybeUninit<BarStruct<'b>>) -> __T
///     {
///         let out = out.as_mut_ptr() as *mut Bar__Variant;
///         unsafe {
///             ::core::ptr::addr_of_mut!((*out).0).write(Foo__Type::Bar__Variant);
///         }
///
///         let item = unsafe { &mut *::core::ptr::addr_of_mut!((*out).1).cast() };
///         init(item)
///     }
/// }
///
/// #[cfg(feature = "baz")]
/// impl Foo<'b> {
///     pub fn init_as_baz<__T, __F>(mut init: __F, out: &mut MaybeUninit<Self>) -> __T
///     where
///         __F: FnMut(&mut MaybeUninit<Baz<'b>>) -> __T
///     {
///         let out = out.as_mut_ptr() as *mut Baz__Variant;
///         unsafe {
///             ::core::ptr::addr_of_mut!((*out).0).write(Foo__Type::Baz__Variant);
///         }
///
///         let item = unsafe { &mut *::core::ptr::addr_of_mut!((*out).1).cast() };
///         init(item)
///     }
/// }
/// ```
pub fn enum_init(metadata: TokenStream, input: TokenStream) -> TokenStream {
    enum_init::enum_init(metadata, input)
}
