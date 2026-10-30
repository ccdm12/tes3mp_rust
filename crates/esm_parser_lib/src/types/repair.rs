use crate::{
    EsmError, EsmReader, RefId,
    esm_record::EsmRecord,
    types::{
        enums::{RecInt},
        flags::ObjectFlags,
    },
};
use esm_parser_macros::LoadSave;

#[derive(Clone, Debug, Default, PartialEq, LoadSave)]
pub struct Repair {
    pub flags: ObjectFlags,
    pub id: RefId,
    pub model: String,
    pub icon: String,
    pub name: String,
    pub data: RepairData,
    pub script: RefId,
}

impl EsmRecord for Repair {
    fn id(&self) -> &RefId {
        &self.id
    }
    const RECORD_ID: RecInt = RecInt::Repa;
}

impl Repair {
    pub fn load(reader: &mut EsmReader, is_deleted: &mut bool) -> Result<Self, EsmError> {
        *is_deleted = false;
        let mut repair = Repair::default();
        repair.flags = reader.get_record_flags();

        let mut has_name = false;
        let mut has_data = false;

        while reader.has_more_subs() {
            let sub_name = reader.get_or_read_sub_name()?;

            match sub_name.as_bytes() {
                b"NAME" => {
                    repair.id = reader.read_refid()?;
                    has_name = true;
                }
                b"MODL" => {
                    let model = reader.read_header_decode_string()?;
                    let normalized = model.replace('\\', "/");
                    repair.model = normalized;
                }
                b"FNAM" => {
                    repair.name = reader.read_header_decode_string()?;
                }
                b"RIDT" => {
                    reader.read_sub_header()?;
                    repair.data = reader.load()?;
                    has_data = true;
                }
                b"SCRI" => {
                    repair.script = reader.read_refid()?;
                }
                b"ITEX" => {
                    repair.icon = reader.read_header_decode_string()?;
                }
                b"DELE" => {
                    reader.skip_sub_header()?;
                    *is_deleted = true;
                }
                _ => {
                    return Err(reader.fail("Unknown subrecord"));
                }
            }
        }

        if !has_name {
            return Err(reader.fail("Missing NAME subrecord"));
        }
        if !has_data && !*is_deleted {
            return Err(reader.fail("Missing RIDT subrecord"));
        }

        Ok(repair)
    }
}

#[derive(Clone, Debug, Default, PartialEq, LoadSave)]
pub struct RepairData {
    pub weight: f32,
    pub value: u32,
    pub uses: u32,
    pub quality: f32,
}
