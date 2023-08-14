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
#[repr(u8)]
/// Represents how to interpret the given data packet for the purpose of uploading
/// some data to the app over multiple packets
pub enum PacketType {
    Init = 0,
    Add = 1,
    Last = 2,
}

impl TryFrom<u8> for PacketType {
    type Error = ();

    fn try_from(from: u8) -> Result<Self, ()> {
        match from {
            0 => Ok(Self::Init),
            1 => Ok(Self::Add),
            2 => Ok(Self::Last),
            _ => Err(()),
        }
    }
}

impl From<PacketType> for u8 {
    fn from(from: PacketType) -> Self {
        from as _
    }
}

impl PacketType {
    #[allow(clippy::result_unit_err)]
    pub fn new(p1: u8) -> Result<Self, ()> {
        Self::try_from(p1)
    }

    pub fn is_init(&self) -> bool {
        matches!(self, Self::Init)
    }

    pub fn is_last(&self) -> bool {
        matches!(self, Self::Last)
    }

    pub fn is_next(&self) -> bool {
        !self.is_init() && !self.is_last()
    }
}
