use crate::{
    EsmError, EsmReader, FixedString, RefId,
    esm_record::EsmRecord,
    types::{enums::RecInt, flags::ObjectFlags},
};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct LevelledListBase {
    pub record_flags: ObjectFlags,
    pub flags: u32,
    pub id: RefId,
    pub chance_none: u8,
    pub list: Vec<LevelItem>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct LevelItem {
    pub id: RefId,
    pub level: u16,
}

impl EsmRecord for LevelledListBase {
    fn id(&self) -> &RefId {
        &self.id
    }
    const RECORD_ID: RecInt = RecInt::Levi;
}

impl LevelledListBase {
    pub fn load_with_rec_name(
        reader: &mut EsmReader,
        is_deleted: &mut bool,
        item_rec_name: &[u8; 4],
    ) -> Result<Self, EsmError> {
        *is_deleted = false;
        let mut levelled_list = LevelledListBase::default();
        levelled_list.record_flags = reader.get_record_flags();

        let mut has_name = false;
        let mut has_list = false;

        while reader.has_more_subs() {
            let sub_name = reader.get_or_read_sub_name()?;

            match sub_name.as_bytes() {
                b"NAME" => {
                    levelled_list.id = reader.read_refid()?;
                    has_name = true;
                }
                b"DATA" => {
                    reader.read_sub_header()?;
                    levelled_list.flags = reader.load()?;
                }
                b"NNAM" => {
                    reader.read_sub_header()?;
                    levelled_list.chance_none = reader.load()?;
                }
                b"INDX" => {
                    reader.read_sub_header()?;
                    let length: u32 = reader.load()?;
                    levelled_list
                        .list
                        .resize(length as usize, LevelItem::default());

                    for item in &mut levelled_list.list {
                        // Validate and read each item sub-record (CNAM or INAM) using read_header_refid_check_name equivalent
                        item.id = reader.read_header_refid_check_name(FixedString {
                            data: *item_rec_name,
                        })?;
                        // Read INTV subrecord for the level
                        item.level = reader.read_typed_named_header(crate::name4!("INTV"))?;
                    }

                    has_list = true;
                }
                b"DELE" => {
                    reader.skip_sub_header()?;
                    *is_deleted = true;
                }
                _ => {
                    if !has_list {
                        levelled_list.list.clear();
                        reader.skip_record()?;
                    } else {
                        return Err(reader.fail("Unknown subrecord"));
                    }
                }
            }
        }

        if !has_name {
            return Err(reader.fail("Missing NAME subrecord"));
        }

        Ok(levelled_list)
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CreatureLevList {
    pub base: LevelledListBase,
}

impl EsmRecord for CreatureLevList {
    fn id(&self) -> &RefId {
        self.base.id()
    }
    const RECORD_ID: RecInt = RecInt::Crec;
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ItemLevList {
    pub base: LevelledListBase,
}

impl EsmRecord for ItemLevList {
    fn id(&self) -> &RefId {
        self.base.id()
    }
    const RECORD_ID: RecInt = RecInt::Levi;
}

impl CreatureLevList {
    pub fn load(reader: &mut EsmReader, is_deleted: &mut bool) -> Result<Self, EsmError> {
        let base = LevelledListBase::load_with_rec_name(reader, is_deleted, b"CNAM")?;
        Ok(CreatureLevList { base })
    }
}

impl ItemLevList {
    pub fn load(reader: &mut EsmReader, is_deleted: &mut bool) -> Result<Self, EsmError> {
        let base = LevelledListBase::load_with_rec_name(reader, is_deleted, b"INAM")?;
        Ok(ItemLevList { base })
    }
}
