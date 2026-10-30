use crate::{
    EsmError, EsmReader, RefId,
    esm_record::EsmRecord,
    types::{
        enums::RecInt,
        flags::ObjectFlags,
        variant::{Variant, VariantFormat},
    },
};

#[derive(Clone, Debug, Default)]
pub struct Global {
    pub record_flags: ObjectFlags,
    pub id: RefId,
    pub value: Variant,
}

impl EsmRecord for Global {
    fn id(&self) -> &RefId {
        &self.id
    }
    const RECORD_ID: RecInt = RecInt::Glob;
}

impl Global {
    pub fn load(reader: &mut EsmReader, is_deleted: &mut bool) -> Result<Self, EsmError> {
        *is_deleted = false;
        let mut global = Global::default();
        global.record_flags = reader.get_record_flags();

        // 1. Read NAME subrecord (ID)[cite: 5, 8]
        global.id = reader.read_header_refid_check_name(crate::name4!("NAME"))?;

        // 2. Check for DELE or read global variant data (which handles FNAM & FLTV)[cite: 5]
        if reader.is_next_sub(crate::name4!("DELE"))? {
            reader.skip_sub_header()?;
            *is_deleted = true;
        } else {
            let (value, _var_type) = Variant::read(reader, VariantFormat::Global)?;
            global.value = value;
        }

        Ok(global)
    }

    pub fn blank(&mut self) {
        self.record_flags = ObjectFlags::empty();
        self.value = Variant::None;
    }
}
