use std::io::{self, Cursor, Read, Write};
use std::collections::HashSet;

pub const MAGIC: u32 = 0xCAFEBABE;

#[derive(Debug, Clone)]
pub enum CpEntry {
    Utf8(String),
    Integer(i32),
    Float(f32),
    Long(i64),
    Double(f64),
    Class { name_index: u16 },
    String { string_index: u16 },
    Fieldref { class_index: u16, name_and_type_index: u16 },
    Methodref { class_index: u16, name_and_type_index: u16 },
    InterfaceMethodref { class_index: u16, name_and_type_index: u16 },
    NameAndType { name_index: u16, descriptor_index: u16 },
    MethodHandle { reference_kind: u8, reference_index: u16 },
    MethodType { descriptor_index: u16 },
    Dynamic { bootstrap_method_attr_index: u16, name_and_type_index: u16 },
    InvokeDynamic { bootstrap_method_attr_index: u16, name_and_type_index: u16 },
    Module { name_index: u16 },
    Package { name_index: u16 },
    Empty, // слот после Long/Double
}

pub struct ClassFile {
    pub minor_version: u16,
    pub major_version: u16,
    pub constant_pool: Vec<CpEntry>, // 1-based index, slot 0 is empty
    pub remainder: Vec<u8>,          // остальная часть байткода (flags, this, super, fields, methods, attrs)
}

impl ClassFile {
    pub fn parse(bytes: &[u8]) -> io::Result<Self> {
        let mut cursor = Cursor::new(bytes);

        let mut magic_buf = [0u8; 4];
        cursor.read_exact(&mut magic_buf)?;
        let magic = u32::from_be_bytes(magic_buf);
        if magic != MAGIC {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "Invalid class magic"));
        }

        let mut u16_buf = [0u8; 2];
        cursor.read_exact(&mut u16_buf)?;
        let minor_version = u16::from_be_bytes(u16_buf);

        cursor.read_exact(&mut u16_buf)?;
        let major_version = u16::from_be_bytes(u16_buf);

        cursor.read_exact(&mut u16_buf)?;
        let cp_count = u16::from_be_bytes(u16_buf) as usize;

        let mut constant_pool = Vec::with_capacity(cp_count);
        constant_pool.push(CpEntry::Empty); // 0 index is unused

        let mut i = 1;
        while i < cp_count {
            let mut tag_buf = [0u8; 1];
            cursor.read_exact(&mut tag_buf)?;
            let tag = tag_buf[0];

            match tag {
                1 => {
                    // CONSTANT_Utf8
                    cursor.read_exact(&mut u16_buf)?;
                    let len = u16::from_be_bytes(u16_buf) as usize;
                    let mut str_bytes = vec![0u8; len];
                    cursor.read_exact(&mut str_bytes)?;
                    let s = String::from_utf8_lossy(&str_bytes).to_string();
                    constant_pool.push(CpEntry::Utf8(s));
                    i += 1;
                }
                3 => {
                    // CONSTANT_Integer
                    let mut buf = [0u8; 4];
                    cursor.read_exact(&mut buf)?;
                    constant_pool.push(CpEntry::Integer(i32::from_be_bytes(buf)));
                    i += 1;
                }
                4 => {
                    // CONSTANT_Float
                    let mut buf = [0u8; 4];
                    cursor.read_exact(&mut buf)?;
                    constant_pool.push(CpEntry::Float(f32::from_be_bytes(buf)));
                    i += 1;
                }
                5 => {
                    // CONSTANT_Long
                    let mut buf = [0u8; 8];
                    cursor.read_exact(&mut buf)?;
                    constant_pool.push(CpEntry::Long(i64::from_be_bytes(buf)));
                    constant_pool.push(CpEntry::Empty);
                    i += 2;
                }
                6 => {
                    // CONSTANT_Double
                    let mut buf = [0u8; 8];
                    cursor.read_exact(&mut buf)?;
                    constant_pool.push(CpEntry::Double(f64::from_be_bytes(buf)));
                    constant_pool.push(CpEntry::Empty);
                    i += 2;
                }
                7 => {
                    // CONSTANT_Class
                    cursor.read_exact(&mut u16_buf)?;
                    constant_pool.push(CpEntry::Class {
                        name_index: u16::from_be_bytes(u16_buf),
                    });
                    i += 1;
                }
                8 => {
                    // CONSTANT_String
                    cursor.read_exact(&mut u16_buf)?;
                    constant_pool.push(CpEntry::String {
                        string_index: u16::from_be_bytes(u16_buf),
                    });
                    i += 1;
                }
                9 => {
                    // CONSTANT_Fieldref
                    let mut buf = [0u8; 4];
                    cursor.read_exact(&mut buf)?;
                    constant_pool.push(CpEntry::Fieldref {
                        class_index: u16::from_be_bytes([buf[0], buf[1]]),
                        name_and_type_index: u16::from_be_bytes([buf[2], buf[3]]),
                    });
                    i += 1;
                }
                10 => {
                    // CONSTANT_Methodref
                    let mut buf = [0u8; 4];
                    cursor.read_exact(&mut buf)?;
                    constant_pool.push(CpEntry::Methodref {
                        class_index: u16::from_be_bytes([buf[0], buf[1]]),
                        name_and_type_index: u16::from_be_bytes([buf[2], buf[3]]),
                    });
                    i += 1;
                }
                11 => {
                    // CONSTANT_InterfaceMethodref
                    let mut buf = [0u8; 4];
                    cursor.read_exact(&mut buf)?;
                    constant_pool.push(CpEntry::InterfaceMethodref {
                        class_index: u16::from_be_bytes([buf[0], buf[1]]),
                        name_and_type_index: u16::from_be_bytes([buf[2], buf[3]]),
                    });
                    i += 1;
                }
                12 => {
                    // CONSTANT_NameAndType
                    let mut buf = [0u8; 4];
                    cursor.read_exact(&mut buf)?;
                    constant_pool.push(CpEntry::NameAndType {
                        name_index: u16::from_be_bytes([buf[0], buf[1]]),
                        descriptor_index: u16::from_be_bytes([buf[2], buf[3]]),
                    });
                    i += 1;
                }
                15 => {
                    // CONSTANT_MethodHandle
                    let mut kind_buf = [0u8; 1];
                    cursor.read_exact(&mut kind_buf)?;
                    cursor.read_exact(&mut u16_buf)?;
                    constant_pool.push(CpEntry::MethodHandle {
                        reference_kind: kind_buf[0],
                        reference_index: u16::from_be_bytes(u16_buf),
                    });
                    i += 1;
                }
                16 => {
                    // CONSTANT_MethodType
                    cursor.read_exact(&mut u16_buf)?;
                    constant_pool.push(CpEntry::MethodType {
                        descriptor_index: u16::from_be_bytes(u16_buf),
                    });
                    i += 1;
                }
                17 => {
                    // CONSTANT_Dynamic
                    let mut buf = [0u8; 4];
                    cursor.read_exact(&mut buf)?;
                    constant_pool.push(CpEntry::Dynamic {
                        bootstrap_method_attr_index: u16::from_be_bytes([buf[0], buf[1]]),
                        name_and_type_index: u16::from_be_bytes([buf[2], buf[3]]),
                    });
                    i += 1;
                }
                18 => {
                    // CONSTANT_InvokeDynamic
                    let mut buf = [0u8; 4];
                    cursor.read_exact(&mut buf)?;
                    constant_pool.push(CpEntry::InvokeDynamic {
                        bootstrap_method_attr_index: u16::from_be_bytes([buf[0], buf[1]]),
                        name_and_type_index: u16::from_be_bytes([buf[2], buf[3]]),
                    });
                    i += 1;
                }
                19 => {
                    // CONSTANT_Module
                    cursor.read_exact(&mut u16_buf)?;
                    constant_pool.push(CpEntry::Module {
                        name_index: u16::from_be_bytes(u16_buf),
                    });
                    i += 1;
                }
                20 => {
                    // CONSTANT_Package
                    cursor.read_exact(&mut u16_buf)?;
                    constant_pool.push(CpEntry::Package {
                        name_index: u16::from_be_bytes(u16_buf),
                    });
                    i += 1;
                }
                unknown => {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("Unknown constant pool tag: {}", unknown),
                    ));
                }
            }
        }

        let current_pos = cursor.position() as usize;
        let remainder = bytes[current_pos..].to_vec();

        Ok(ClassFile {
            minor_version,
            major_version,
            constant_pool,
            remainder,
        })
    }

    /// Собрать класс обратно в байты
    pub fn write_to_bytes(&self) -> io::Result<Vec<u8>> {
        let mut out = Vec::with_capacity(1024 + self.remainder.len());
        out.write_all(&MAGIC.to_be_bytes())?;
        out.write_all(&self.minor_version.to_be_bytes())?;
        out.write_all(&self.major_version.to_be_bytes())?;
        out.write_all(&(self.constant_pool.len() as u16).to_be_bytes())?;

        let mut i = 1;
        while i < self.constant_pool.len() {
            match &self.constant_pool[i] {
                CpEntry::Utf8(s) => {
                    out.write_all(&[1])?;
                    let bytes = s.as_bytes();
                    out.write_all(&(bytes.len() as u16).to_be_bytes())?;
                    out.write_all(bytes)?;
                    i += 1;
                }
                CpEntry::Integer(v) => {
                    out.write_all(&[3])?;
                    out.write_all(&v.to_be_bytes())?;
                    i += 1;
                }
                CpEntry::Float(v) => {
                    out.write_all(&[4])?;
                    out.write_all(&v.to_be_bytes())?;
                    i += 1;
                }
                CpEntry::Long(v) => {
                    out.write_all(&[5])?;
                    out.write_all(&v.to_be_bytes())?;
                    i += 2;
                }
                CpEntry::Double(v) => {
                    out.write_all(&[6])?;
                    out.write_all(&v.to_be_bytes())?;
                    i += 2;
                }
                CpEntry::Class { name_index } => {
                    out.write_all(&[7])?;
                    out.write_all(&name_index.to_be_bytes())?;
                    i += 1;
                }
                CpEntry::String { string_index } => {
                    out.write_all(&[8])?;
                    out.write_all(&string_index.to_be_bytes())?;
                    i += 1;
                }
                CpEntry::Fieldref { class_index, name_and_type_index } => {
                    out.write_all(&[9])?;
                    out.write_all(&class_index.to_be_bytes())?;
                    out.write_all(&name_and_type_index.to_be_bytes())?;
                    i += 1;
                }
                CpEntry::Methodref { class_index, name_and_type_index } => {
                    out.write_all(&[10])?;
                    out.write_all(&class_index.to_be_bytes())?;
                    out.write_all(&name_and_type_index.to_be_bytes())?;
                    i += 1;
                }
                CpEntry::InterfaceMethodref { class_index, name_and_type_index } => {
                    out.write_all(&[11])?;
                    out.write_all(&class_index.to_be_bytes())?;
                    out.write_all(&name_and_type_index.to_be_bytes())?;
                    i += 1;
                }
                CpEntry::NameAndType { name_index, descriptor_index } => {
                    out.write_all(&[12])?;
                    out.write_all(&name_index.to_be_bytes())?;
                    out.write_all(&descriptor_index.to_be_bytes())?;
                    i += 1;
                }
                CpEntry::MethodHandle { reference_kind, reference_index } => {
                    out.write_all(&[15, *reference_kind])?;
                    out.write_all(&reference_index.to_be_bytes())?;
                    i += 1;
                }
                CpEntry::MethodType { descriptor_index } => {
                    out.write_all(&[16])?;
                    out.write_all(&descriptor_index.to_be_bytes())?;
                    i += 1;
                }
                CpEntry::Dynamic { bootstrap_method_attr_index, name_and_type_index } => {
                    out.write_all(&[17])?;
                    out.write_all(&bootstrap_method_attr_index.to_be_bytes())?;
                    out.write_all(&name_and_type_index.to_be_bytes())?;
                    i += 1;
                }
                CpEntry::InvokeDynamic { bootstrap_method_attr_index, name_and_type_index } => {
                    out.write_all(&[18])?;
                    out.write_all(&bootstrap_method_attr_index.to_be_bytes())?;
                    out.write_all(&name_and_type_index.to_be_bytes())?;
                    i += 1;
                }
                CpEntry::Module { name_index } => {
                    out.write_all(&[19])?;
                    out.write_all(&name_index.to_be_bytes())?;
                    i += 1;
                }
                CpEntry::Package { name_index } => {
                    out.write_all(&[20])?;
                    out.write_all(&name_index.to_be_bytes())?;
                    i += 1;
                }
                CpEntry::Empty => {
                    i += 1;
                }
            }
        }

        out.write_all(&self.remainder)?;
        Ok(out)
    }

    /// Извлечь все имена классов, на которые ссылается данный класс (для Tree Shaking)
    pub fn extract_referenced_classes(&self) -> HashSet<String> {
        let mut refs = HashSet::new();
        for entry in &self.constant_pool {
            if let CpEntry::Class { name_index } = entry {
                if let Some(CpEntry::Utf8(name)) = self.constant_pool.get(*name_index as usize) {
                    // Игнорируем массивы примитивов типа [B, [I
                    let clean = name.trim_start_matches('[');
                    if let Some(stripped) = clean.strip_prefix('L') {
                        refs.insert(stripped.trim_end_matches(';').to_string());
                    } else if !clean.is_empty() {
                        refs.insert(clean.to_string());
                    }
                }
            }
        }
        refs
    }

    /// Получить имя текущего класса из remainder (this_class: u16)
    pub fn get_this_class_name(&self) -> Option<String> {
        if self.remainder.len() >= 4 {
            let this_class_idx = u16::from_be_bytes([self.remainder[2], self.remainder[3]]) as usize;
            if let Some(CpEntry::Class { name_index }) = self.constant_pool.get(this_class_idx) {
                if let Some(CpEntry::Utf8(name)) = self.constant_pool.get(*name_index as usize) {
                    return Some(name.clone());
                }
            }
        }
        None
    }
}
