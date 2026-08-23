//! Dataset definitions, strongly-typed domain enums, disk loading, and pre-flight validation.

use std::fmt;
use std::fs;
use std::path::Path;
use serde::{Deserialize, Serialize};
use crate::engine::{decompress_bz2_multistream_into, decompress_bz2_single_into};

/// Semantic category of the data payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DataCategory {
    AsciiText,
    TarExecutable,
    MedicalMri,
    MedicalXRay,
    ChemistryDb,
    X86Executable,
    DatabaseBinary,
    DocumentPdf,
    SourceCode,
    BinaryCatalog,
    XmlMarkup,
    RadarArchive,
}

impl fmt::Display for DataCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            DataCategory::AsciiText => "Text (ASCII)",
            DataCategory::TarExecutable => "Tar / Executables",
            DataCategory::MedicalMri => "Medical (MRI)",
            DataCategory::MedicalXRay => "Medical (X-Ray)",
            DataCategory::ChemistryDb => "Text (Chem DB)",
            DataCategory::X86Executable => "x86 Executable",
            DataCategory::DatabaseBinary => "DB Binary",
            DataCategory::DocumentPdf => "PDF Document",
            DataCategory::SourceCode => "Tar / C Source",
            DataCategory::BinaryCatalog => "Binary Catalog",
            DataCategory::XmlMarkup => "XML Markup",
            DataCategory::RadarArchive => "Radar Binary Archive",
        };
        write!(f, "{}", label)
    }
}

/// Enumeration of canonical Silesia Corpus benchmark files.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SilesiaFileId {
    Dickens,
    Mozilla,
    Mr,
    Nci,
    Ooffice,
    Osdb,
    Reymont,
    Samba,
    Sao,
    Webster,
    Xml,
    XRay,
}

impl SilesiaFileId {
    pub const fn filename(self) -> &'static str {
        match self {
            SilesiaFileId::Dickens => "silesia_dickens",
            SilesiaFileId::Mozilla => "silesia_mozilla",
            SilesiaFileId::Mr => "silesia_mr",
            SilesiaFileId::Nci => "silesia_nci",
            SilesiaFileId::Ooffice => "silesia_ooffice",
            SilesiaFileId::Osdb => "silesia_osdb",
            SilesiaFileId::Reymont => "silesia_reymont",
            SilesiaFileId::Samba => "silesia_samba",
            SilesiaFileId::Sao => "silesia_sao",
            SilesiaFileId::Webster => "silesia_webster",
            SilesiaFileId::Xml => "silesia_xml",
            SilesiaFileId::XRay => "silesia_x-ray",
        }
    }

    pub const fn short_name(self) -> &'static str {
        match self {
            SilesiaFileId::Dickens => "dickens",
            SilesiaFileId::Mozilla => "mozilla",
            SilesiaFileId::Mr => "mr",
            SilesiaFileId::Nci => "nci",
            SilesiaFileId::Ooffice => "ooffice",
            SilesiaFileId::Osdb => "osdb",
            SilesiaFileId::Reymont => "reymont",
            SilesiaFileId::Samba => "samba",
            SilesiaFileId::Sao => "sao",
            SilesiaFileId::Webster => "webster",
            SilesiaFileId::Xml => "xml",
            SilesiaFileId::XRay => "x-ray",
        }
    }

    pub const fn category(self) -> DataCategory {
        match self {
            SilesiaFileId::Dickens => DataCategory::AsciiText,
            SilesiaFileId::Mozilla => DataCategory::TarExecutable,
            SilesiaFileId::Mr => DataCategory::MedicalMri,
            SilesiaFileId::Nci => DataCategory::ChemistryDb,
            SilesiaFileId::Ooffice => DataCategory::X86Executable,
            SilesiaFileId::Osdb => DataCategory::DatabaseBinary,
            SilesiaFileId::Reymont => DataCategory::DocumentPdf,
            SilesiaFileId::Samba => DataCategory::SourceCode,
            SilesiaFileId::Sao => DataCategory::BinaryCatalog,
            SilesiaFileId::Webster => DataCategory::AsciiText,
            SilesiaFileId::Xml => DataCategory::XmlMarkup,
            SilesiaFileId::XRay => DataCategory::MedicalXRay,
        }
    }
}

/// Canonical metadata for a benchmark test file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileMeta {
    pub id: SilesiaFileId,
    pub name: &'static str,
    pub category: DataCategory,
}

/// Canonical 12-file Silesia Compression Corpus list.
pub const SILESIA_FILES: &[FileMeta] = &[
    FileMeta { id: SilesiaFileId::Dickens, name: SilesiaFileId::Dickens.filename(), category: SilesiaFileId::Dickens.category() },
    FileMeta { id: SilesiaFileId::Mozilla, name: SilesiaFileId::Mozilla.filename(), category: SilesiaFileId::Mozilla.category() },
    FileMeta { id: SilesiaFileId::Mr, name: SilesiaFileId::Mr.filename(), category: SilesiaFileId::Mr.category() },
    FileMeta { id: SilesiaFileId::Nci, name: SilesiaFileId::Nci.filename(), category: SilesiaFileId::Nci.category() },
    FileMeta { id: SilesiaFileId::Ooffice, name: SilesiaFileId::Ooffice.filename(), category: SilesiaFileId::Ooffice.category() },
    FileMeta { id: SilesiaFileId::Osdb, name: SilesiaFileId::Osdb.filename(), category: SilesiaFileId::Osdb.category() },
    FileMeta { id: SilesiaFileId::Reymont, name: SilesiaFileId::Reymont.filename(), category: SilesiaFileId::Reymont.category() },
    FileMeta { id: SilesiaFileId::Samba, name: SilesiaFileId::Samba.filename(), category: SilesiaFileId::Samba.category() },
    FileMeta { id: SilesiaFileId::Sao, name: SilesiaFileId::Sao.filename(), category: SilesiaFileId::Sao.category() },
    FileMeta { id: SilesiaFileId::Webster, name: SilesiaFileId::Webster.filename(), category: SilesiaFileId::Webster.category() },
    FileMeta { id: SilesiaFileId::Xml, name: SilesiaFileId::Xml.filename(), category: SilesiaFileId::Xml.category() },
    FileMeta { id: SilesiaFileId::XRay, name: SilesiaFileId::XRay.filename(), category: SilesiaFileId::XRay.category() },
];

/// Preloaded in-memory representation of a benchmark test case.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetItem {
    pub name: String,
    pub category: DataCategory,
    #[serde(skip)]
    pub uncompressed: Vec<u8>,
    #[serde(skip)]
    pub compressed: Vec<u8>,
}

/// Load and pre-flight validate Silesia Corpus files into memory.
pub fn load_silesia(ref_dir: &Path, comp_dir: &Path, files: &[FileMeta]) -> Result<Vec<DatasetItem>, Box<dyn std::error::Error>> {
    let mut items = Vec::new();
    let mut test_buf = vec![0u8; 64 * 1024 * 1024];

    for f in files {
        let ref_path = ref_dir.join(f.name);
        let comp_path = comp_dir.join(format!("{}.bz2", f.name));
        let uncompressed = fs::read(&ref_path)
            .map_err(|e| format!("Failed to read reference file {}: {}", ref_path.display(), e))?;
        let compressed = fs::read(&comp_path)
            .map_err(|e| format!("Failed to read compressed file {}: {}", comp_path.display(), e))?;

        let decomp_len = decompress_bz2_single_into(&compressed, &mut test_buf)
            .map_err(|e| format!("Validation failure on {}: {}", f.name, e))?;
        assert_eq!(decomp_len, uncompressed.len(), "Length mismatch on {}", f.name);
        assert_eq!(&test_buf[..decomp_len], &uncompressed[..], "Content mismatch on {}", f.name);

        items.push(DatasetItem {
            name: f.name.to_string(),
            category: f.category,
            uncompressed,
            compressed,
        });
    }
    Ok(items)
}

/// Load and pre-flight validate 30 NOAA NEXRAD Radar volume archives into memory.
pub fn load_nexrad(ref_dir: &Path, comp_dir: &Path) -> Result<Vec<DatasetItem>, Box<dyn std::error::Error>> {
    let mut items = Vec::new();
    let mut test_buf = vec![0u8; 64 * 1024 * 1024];

    for i in 1..=30 {
        let name = format!("nexrad{}", i);
        let ref_path = ref_dir.join(&name);
        let comp_path = comp_dir.join(format!("{}.bz2", name));
        let uncompressed = fs::read(&ref_path)
            .map_err(|e| format!("Failed to read reference file {}: {}", ref_path.display(), e))?;
        let compressed = fs::read(&comp_path)
            .map_err(|e| format!("Failed to read compressed file {}: {}", comp_path.display(), e))?;

        let decomp_len = decompress_bz2_multistream_into(&compressed, &mut test_buf)
            .map_err(|e| format!("Validation failure on {}: {}", name, e))?;
        assert_eq!(decomp_len, uncompressed.len(), "Length mismatch on {}", name);
        assert_eq!(&test_buf[..decomp_len], &uncompressed[..], "Content mismatch on {}", name);

        items.push(DatasetItem {
            name,
            category: DataCategory::RadarArchive,
            uncompressed,
            compressed,
        });
    }
    Ok(items)
}
