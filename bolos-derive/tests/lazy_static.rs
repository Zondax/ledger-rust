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

*******************************************************************************/
use bolos::pic::PIC;
use bolos_derive::*;

#[lazy_static]
static mut SOMETHING: u32 = 33;

#[test]
fn check_lazy() {
    let something: &mut __IMPL_LAZY_SOMETHING::__LAZY_SOMETHING = unsafe { &mut SOMETHING };
    let something = &mut **something;

    assert_eq!(33, *something);

    *something += 1;

    assert_eq!(34, *something);
}
