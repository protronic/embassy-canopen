use core::usize;

use heapless::FnvIndexMap;

#[allow(unused)]
pub struct ObjectDictionaryEntry {
    index: u16,
    subindex: u8,
    data_type: DataType,
    access_type: AccessType,
    pub(crate) value: Value,
}

#[allow(unused)]
pub enum DataType {
    Boolean,
    Integer8,
    Integer16,
    Integer32,
    Unsigned8,
    Unsigned16,
    Unsigned32,
    Float32,
}

#[allow(unused)]
pub enum AccessType {
    ReadOnly,
    WriteOnly,
    ReadWrite,
}

#[allow(unused)]
#[derive(Copy, Clone, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Value {
    Bool(bool),
    Int8(i8),
    Int16(i16),
    Int32(i32),
    Uint8(u8),
    Uint16(u16),
    Uint32(u32),
    Float32(f32),
}

#[derive(Copy, Clone, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum ReadWriteError {
    // Cannot read from a write-only entry.
    AccessDenied,
}

impl ObjectDictionaryEntry {
    #[allow(unused)]
    fn read(&self) -> Result<Value, ReadWriteError> {
        if matches!(
            self.access_type,
            AccessType::ReadOnly | AccessType::ReadWrite
        ) {
            Ok(self.value)
        } else {
            Err(ReadWriteError::AccessDenied)
        }
    }

    #[allow(unused)]
    fn write(&mut self, new_value: Value) -> Result<(), ReadWriteError> {
        if matches!(
            self.access_type,
            AccessType::WriteOnly | AccessType::ReadWrite
        ) {
            self.value = new_value;
            Ok(())
        } else {
            Err(ReadWriteError::AccessDenied)
        }
    }
}


pub struct Config {
    
}

impl Default for Config {
    fn default() -> Self {
        Self {
        }
    }
}

type ObjectDictionaryEntryId = (u16, u8);

#[allow(unused)]
pub struct ObjectDictionary<const N: usize> {
    entries: FnvIndexMap<ObjectDictionaryEntryId, ObjectDictionaryEntry, N>,
}

impl<const N: usize> ObjectDictionary<N> {
    #[allow(unused)]
    pub fn add_entry(&mut self, entry: ObjectDictionaryEntry) {
        self.entries.insert((entry.index, entry.subindex), entry);
    }

    #[allow(unused)]
    pub fn get_entry(&self, index: u16, subindex: u8) -> Option<&ObjectDictionaryEntry> {
        self.entries.get(&(index, subindex))
    }

    fn new() -> Self {
        Self {
            entries: FnvIndexMap::new(),
        }
    }

    #[allow(unused)]
    pub fn new_canopen_301(_config: Config) -> Self {
        let mut od = Self::new();

        // Example entries as per CANopen 301
        // Device Type (Index 0x1000)
        od.add_entry(ObjectDictionaryEntry {
            index: 0x1000,
            subindex: 0,
            data_type: DataType::Unsigned32,
            access_type: AccessType::ReadOnly,
            value: Value::Uint32(0x00000000), // Replace with actual device type
        });

        // Error Register (Index 0x1001)
        od.add_entry(ObjectDictionaryEntry {
            index: 0x1001,
            subindex: 0,
            data_type: DataType::Unsigned8,
            access_type: AccessType::ReadOnly,
            value: Value::Uint8(0), // Replace with actual error register
        });

        // Manufacturer Status Register (Index 0x1002) - optional
        od.add_entry(ObjectDictionaryEntry {
            index: 0x1002,
            subindex: 0,
            data_type: DataType::Unsigned32,
            access_type: AccessType::ReadOnly,
            value: Value::Uint32(0), // Replace with actual status register
        });

        // Pre-defined error field (Index 0x1003) - Error history (optional)
        od.add_entry(ObjectDictionaryEntry {
            index: 0x1003,
            subindex: 0,
            data_type: DataType::Unsigned32,
            access_type: AccessType::ReadOnly,
            value: Value::Uint32(0), // Error history placeholder
        });

        // COB-ID SYNC Message (Index 0x1005)
        od.add_entry(ObjectDictionaryEntry {
            index: 0x1005,
            subindex: 0,
            data_type: DataType::Unsigned32,
            access_type: AccessType::ReadWrite,
            value: Value::Uint32(0x40000000), // Default COB-ID for SYNC
        });

        // Communication cycle period (Index 0x1006)
        od.add_entry(ObjectDictionaryEntry {
            index: 0x1006,
            subindex: 0,
            data_type: DataType::Unsigned32,
            access_type: AccessType::ReadWrite,
            value: Value::Uint32(0), // Optional, 0 = no sync period
        });

        // Heartbeat Producer Time (Index 0x1017)
        od.add_entry(ObjectDictionaryEntry {
            index: 0x1017,
            subindex: 0,
            data_type: DataType::Unsigned16,
            access_type: AccessType::ReadWrite,
            value: Value::Uint16(1000), // Default to 1000ms
        });

        od
    }
}