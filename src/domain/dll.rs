use std::{fs::File, path::Path};

use anyhow::{Context, Result};
use goblin::pe::{
    export::{ExportAddressTableEntry, Reexport},
    options::ParseOptions,
    utils, PE,
};
use memmap2::MmapOptions;

#[derive(Debug, Clone)]
pub struct ExportEntry {
    pub name: String,
    pub ordinal: u16,
    pub forwarder: Option<String>,
    pub is_ordinal_only: bool,
}

impl ExportEntry {
    pub fn named(name: impl Into<String>, ordinal: u16, forwarder: Option<String>) -> Self {
        Self {
            name: name.into(),
            ordinal,
            forwarder,
            is_ordinal_only: false,
        }
    }

    pub fn ordinal_only(ordinal: u16, forwarder: Option<String>) -> Self {
        Self {
            name: format!("#{ordinal}"),
            ordinal,
            forwarder,
            is_ordinal_only: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DllExports {
    pub arch: String,
    pub exports: Vec<ExportEntry>,
}

pub fn read_exports(path: &Path) -> Result<DllExports> {
    let file =
        File::open(path).with_context(|| format!("Failed to open DLL: {}", path.display()))?;
    let mmap = unsafe {
        MmapOptions::new()
            .map(&file)
            .with_context(|| format!("Failed to mmap DLL: {}", path.display()))?
    };

    let pe = PE::parse(&mmap).with_context(|| format!("Failed to parse PE: {}", path.display()))?;
    let export_data = pe
        .export_data
        .as_ref()
        .context("DLL missing export table")?;
    let ordinal_base = export_data.export_directory_table.ordinal_base;
    let arch = if pe.is_64 { "x64" } else { "x86" }.to_string();
    let file_alignment = pe
        .header
        .optional_header
        .as_ref()
        .context("PE missing optional header")?
        .windows_fields
        .file_alignment;
    let parse_opts = ParseOptions::default();

    let exports = collect_exports(
        &mmap,
        &pe,
        export_data,
        file_alignment,
        ordinal_base,
        &parse_opts,
    )?;

    Ok(DllExports { arch, exports })
}

fn collect_exports(
    bytes: &[u8],
    pe: &PE<'_>,
    export_data: &goblin::pe::export::ExportData<'_>,
    file_alignment: u32,
    ordinal_base: u32,
    parse_opts: &ParseOptions,
) -> Result<Vec<ExportEntry>> {
    let mut names_by_index = vec![Vec::<String>::new(); export_data.export_address_table.len()];

    for (&name_rva, &ordinal_index) in export_data
        .export_name_pointer_table
        .iter()
        .zip(export_data.export_ordinal_table.iter())
    {
        let Some(names) = names_by_index.get_mut(ordinal_index as usize) else {
            continue;
        };

        let Some(name_offset) =
            utils::find_offset(name_rva as usize, &pe.sections, file_alignment, parse_opts)
        else {
            continue;
        };

        let Some(name) = read_pe_string(bytes, name_offset) else {
            continue;
        };

        if !name.is_empty() {
            names.push(name);
        }
    }

    let mut exports = Vec::new();
    for (address_index, address_entry) in export_data.export_address_table.iter().enumerate() {
        if matches!(address_entry, ExportAddressTableEntry::ExportRVA(0)) {
            continue;
        }

        let ordinal = ordinal_base
            .saturating_add(address_index as u32)
            .min(u16::MAX as u32) as u16;
        let forwarder = forwarder_for_entry(bytes, pe, file_alignment, address_entry, parse_opts)?;

        match names_by_index.get(address_index) {
            Some(names) if !names.is_empty() => {
                for name in names {
                    exports.push(ExportEntry::named(name.clone(), ordinal, forwarder.clone()));
                }
            }
            _ => exports.push(ExportEntry::ordinal_only(ordinal, forwarder)),
        }
    }

    Ok(exports)
}

fn read_pe_string(bytes: &[u8], offset: usize) -> Option<String> {
    let tail = bytes.get(offset..)?;
    let end = tail.iter().position(|&b| b == 0)?;
    Some(String::from_utf8_lossy(&tail[..end]).into_owned())
}

fn forwarder_for_entry(
    bytes: &[u8],
    pe: &PE<'_>,
    file_alignment: u32,
    address_entry: &ExportAddressTableEntry,
    parse_opts: &ParseOptions,
) -> Result<Option<String>> {
    let ExportAddressTableEntry::ForwarderRVA(rva) = address_entry else {
        return Ok(None);
    };

    let offset = utils::find_offset_or(
        *rva as usize,
        &pe.sections,
        file_alignment,
        parse_opts,
        "Cannot map forwarder RVA into file offset",
    )
    .map_err(anyhow::Error::from)?;

    let reexport = Reexport::parse(bytes, offset).map_err(anyhow::Error::from)?;
    Ok(Some(match reexport {
        Reexport::DLLName { lib, export } => format!("{lib}!{export}"),
        Reexport::DLLOrdinal { lib, ordinal } => format!("{lib}!#{ordinal}"),
    }))
}

#[cfg(test)]
mod tests {
    use super::ExportEntry;

    #[test]
    fn constructors_preserve_named_vs_ordinal_only_exports() {
        let named = ExportEntry::named("#RealName", 12, None);
        assert_eq!(named.name, "#RealName");
        assert_eq!(named.ordinal, 12);
        assert!(!named.is_ordinal_only);

        let ordinal_only = ExportEntry::ordinal_only(34, Some("other.dll!#7".to_string()));
        assert_eq!(ordinal_only.name, "#34");
        assert_eq!(ordinal_only.ordinal, 34);
        assert!(ordinal_only.is_ordinal_only);
        assert_eq!(ordinal_only.forwarder.as_deref(), Some("other.dll!#7"));
    }
}
