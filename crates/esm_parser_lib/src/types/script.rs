use crate::{
    EsmError, EsmReader, RefId,
    esm_reader::ConsumptionType,
    esm_record::EsmRecord,
    types::{enums::RecInt, flags::ObjectFlags},
};
use esm_parser_macros::LoadSave;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Script {
    pub record_flags: ObjectFlags,
    pub id: RefId,
    pub num_shorts: u32,
    pub num_longs: u32,
    pub num_floats: u32,
    pub var_names: Vec<String>,
    pub script_data: Vec<u8>,
    pub script_text: String,
}

impl EsmRecord for Script {
    fn id(&self) -> &RefId {
        &self.id
    }
    const RECORD_ID: RecInt = RecInt::Scpt;
}

impl Script {
    pub fn load(reader: &mut EsmReader, is_deleted: &mut bool) -> Result<Self, EsmError> {
        *is_deleted = false;
        let mut script = Script::default();
        script.record_flags = reader.get_record_flags();

        let mut header: Option<SCHD> = None;

        while reader.has_more_subs() {
            let sub_name = reader.get_or_read_sub_name()?;

            match sub_name.as_bytes() {
                b"SCHD" => {
                    reader.read_sub_header()?;
                    script.id = reader.get_maybe_fixed_refid_sized(32)?;
                    let schd: SCHD = reader.load()?;
                    script.num_shorts = schd.num_shorts;
                    script.num_longs = schd.num_longs;
                    script.num_floats = schd.num_floats;
                    header = Some(schd);
                }
                b"SCVR" => {
                    let hdr = header
                        .as_ref()
                        .ok_or_else(|| reader.fail("SCVR is placed before SCHD record"))?;
                    load_var_names(hdr, &mut script.var_names, reader)?;
                }
                b"SCDT" => {
                    let hdr = header
                        .as_ref()
                        .ok_or_else(|| reader.fail("SCDT is placed before SCHD record"))?;

                    reader.read_sub_header()?;
                    let sub_size = reader.get_sub_size();

                    if sub_size != hdr.script_data_size {
                        // OpenMW logs a verbose warning here, but proceeds to read the data anyway
                    }

                    script.script_data.resize(sub_size as usize, 0);
                    reader
                        .read_exact(&mut script.script_data, ConsumptionType::SubrecordPayload)?;
                }
                b"SCTX" => {
                    script.script_text = reader.read_header_decode_string()?;
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

        if header.is_none() {
            return Err(reader.fail("Missing SCHD subrecord"));
        }

        Ok(script)
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, LoadSave)]
struct SCHD {
    pub num_shorts: u32,
    pub num_longs: u32,
    pub num_floats: u32,
    pub script_data_size: u32,
    pub string_table_size: u32,
}

fn load_var_names(
    header: &SCHD,
    var_names: &mut Vec<String>,
    reader: &mut EsmReader,
) -> Result<(), EsmError> {
    reader.read_sub_header()?;
    let s = header.string_table_size as usize;
    let mut buffer = vec![0u8; reader.get_sub_size() as usize];

    reader.read_exact(&mut buffer, ConsumptionType::SubrecordPayload)?;

    if buffer.len() < s {
        return Err(reader.fail("SCVR string list is smaller than specified"));
    }

    let mut tmp = buffer[..s].to_vec();

    var_names.resize(
        (header.num_shorts + header.num_longs + header.num_floats) as usize,
        String::new(),
    );

    if tmp.is_empty() {
        return Ok(());
    }

    // Support '\r' terminated strings like vanilla OpenMW
    for byte in tmp.iter_mut() {
        if *byte == b'\r' {
            *byte = 0;
        }
    }

    if *tmp.last().unwrap() != 0 {
        tmp.push(0);
    }

    let mut offset = 0;
    for i in 0..var_names.len() {
        if offset >= tmp.len() {
            var_names.resize(i, String::new());
            break;
        }
        let slice = &tmp[offset..];
        let null_pos = match slice.iter().position(|&x| x == 0) {
            Some(pos) => pos,
            None => {
                var_names.resize(i, String::new());
                break;
            }
        };

        var_names[i] = String::from_utf8_lossy(&slice[..null_pos]).into_owned();
        offset += null_pos + 1;
    }

    Ok(())
}
