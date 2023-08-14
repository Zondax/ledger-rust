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
#![allow(dead_code, unused_macros)]

/// This trait defines a way to `unwrap` or `expect` whilst relaxing the required bounds
/// on the items.
///
/// This is useful to reduce the size of the app, as Debug implementations are
/// not required to be present always
///
/// It is important to remember that using these `unwrap` and `expect` should
/// be done exclusively when there's no way that a panic can happen, for example when
/// unwrapping an option that is guaranteed to be some by an earlier check.
///
/// It makes sense that we declare these as unsafe to make apparent that there is a
/// documentation construct to respect, but doing so would make the ergonomics very poor
pub trait LedgerUnwrap: Sized {
    type Item;

    /// !!! only use when certain there's no panic
    fn ledger_unwrap(self) -> Self::Item;

    /// !!! only use when certain there's no panic
    fn ledger_expect(self, s: &str) -> Self::Item;
}

impl<T, E> LedgerUnwrap for Result<T, E> {
    type Item = T;

    #[inline]
    fn ledger_unwrap(self) -> Self::Item {
        match self {
            Ok(t) => t,
            Err(_) => unsafe { std::hint::unreachable_unchecked() },
        }
    }

    #[inline]
    fn ledger_expect(self, _: &str) -> Self::Item {
        match self {
            Ok(t) => t,
            Err(_) => unsafe { std::hint::unreachable_unchecked() },
        }
    }
}

impl<T> LedgerUnwrap for Option<T> {
    type Item = T;

    #[inline]
    fn ledger_unwrap(self) -> Self::Item {
        match self {
            Some(t) => t,
            None => unsafe { std::hint::unreachable_unchecked() },
        }
    }

    #[inline]
    fn ledger_expect(self, _: &str) -> Self::Item {
        match self {
            Some(t) => t,
            None => unsafe { std::hint::unreachable_unchecked() },
        }
    }
}
