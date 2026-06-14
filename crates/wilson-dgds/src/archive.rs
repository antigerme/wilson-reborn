// SPDX-License-Identifier: GPL-3.0-or-later
//! High-level loader tying `RESOURCE.MAP` to the resources in `RESOURCE.001`.
//!
//! Parses every entry of the index, reads its header from the archive and decodes
//! it into the appropriate typed resource. Unknown types (e.g. `FILES.VIN`) are
//! recorded in [`Archive::skipped`] rather than failing.

use crate::ads::Ads;
use crate::bmp::Bmp;
use crate::error::{DgdsError, Result};
use crate::pal::Palette;
use crate::resource::{read_entry_header, resource_extension, ResourceMap};
use crate::scr::Scr;
use crate::ttm::Ttm;

/// All resources decoded from a `RESOURCE.MAP` + archive pair.
#[derive(Debug, Default)]
pub struct Archive {
    /// Name of the archive file referenced by the index (e.g. `RESOURCE.001`).
    pub data_file_name: String,
    /// Palette resources, as `(name, palette)`.
    pub palettes: Vec<(String, Palette)>,
    /// Bitmap sheets, as `(name, bmp)`.
    pub bitmaps: Vec<(String, Bmp)>,
    /// Full-screen images, as `(name, scr)`.
    pub screens: Vec<(String, Scr)>,
    /// Animation scripts, as `(name, ttm)`.
    pub ttms: Vec<(String, Ttm)>,
    /// Scene-sequencing scripts, as `(name, ads)`.
    pub ads: Vec<(String, Ads)>,
    /// Names of resources that were recognised but not decoded (e.g. `.VIN`).
    pub skipped: Vec<String>,
}

impl Archive {
    /// Parse the index (`map_bytes`) and decode every resource from `archive_bytes`.
    pub fn parse(map_bytes: &[u8], archive_bytes: &[u8]) -> Result<Self> {
        let map = ResourceMap::parse(map_bytes)?;
        let mut archive = Archive {
            data_file_name: map.data_file_name.clone(),
            ..Default::default()
        };

        for entry in &map.entries {
            let header = read_entry_header(archive_bytes, entry.offset)?;
            let body = archive_bytes.get(header.body_offset..).ok_or_else(|| {
                DgdsError::eof(
                    "archive: entry body",
                    header.body_offset,
                    archive_bytes.len(),
                )
            })?;
            let name = header.name.clone();

            match resource_extension(&name).map(str::to_ascii_uppercase) {
                Some(ext) if ext == ".ADS" => archive.ads.push((name, Ads::parse(body)?)),
                Some(ext) if ext == ".BMP" => archive.bitmaps.push((name, Bmp::parse(body)?)),
                Some(ext) if ext == ".PAL" => archive.palettes.push((name, Palette::parse(body)?)),
                Some(ext) if ext == ".SCR" => archive.screens.push((name, Scr::parse(body)?)),
                Some(ext) if ext == ".TTM" => archive.ttms.push((name, Ttm::parse(body)?)),
                _ => archive.skipped.push(name),
            }
        }

        Ok(archive)
    }

    /// Find an `.ADS` resource by name (case-sensitive, e.g. `"FISHING.ADS"`).
    pub fn ads(&self, name: &str) -> Option<&Ads> {
        self.ads.iter().find(|(n, _)| n == name).map(|(_, v)| v)
    }

    /// Find a `.TTM` resource by name.
    pub fn ttm(&self, name: &str) -> Option<&Ttm> {
        self.ttms.iter().find(|(n, _)| n == name).map(|(_, v)| v)
    }

    /// Find a `.BMP` resource by name.
    pub fn bmp(&self, name: &str) -> Option<&Bmp> {
        self.bitmaps.iter().find(|(n, _)| n == name).map(|(_, v)| v)
    }

    /// Find a `.SCR` resource by name.
    pub fn scr(&self, name: &str) -> Option<&Scr> {
        self.screens.iter().find(|(n, _)| n == name).map(|(_, v)| v)
    }

    /// The first (global) palette, if any.
    pub fn palette(&self) -> Option<&Palette> {
        self.palettes.first().map(|(_, p)| p)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 13-byte NUL-padded resource name field.
    fn name_field(name: &str) -> Vec<u8> {
        let mut f = vec![0u8; 13];
        f[..name.len()].copy_from_slice(name.as_bytes());
        f
    }

    fn minimal_pal_body() -> Vec<u8> {
        let mut p = Vec::new();
        p.extend_from_slice(b"PAL:");
        p.extend_from_slice(&0u16.to_le_bytes());
        p.extend_from_slice(&[0, 0]);
        p.extend_from_slice(b"VGA:");
        p.extend_from_slice(&768u32.to_le_bytes());
        p.extend_from_slice(&[63, 0, 0]); // colour 0
        for _ in 1..256 {
            p.extend_from_slice(&[0, 0, 0]);
        }
        p
    }

    #[test]
    fn loads_pal_and_skips_vin() {
        // Build RESOURCE.001 with two entries: TEST.PAL then FILES.VIN.
        let pal_body = minimal_pal_body();
        let mut archive_bytes = Vec::new();

        let pal_offset = archive_bytes.len() as u32;
        archive_bytes.extend_from_slice(&name_field("TEST.PAL"));
        archive_bytes.extend_from_slice(&(pal_body.len() as u32).to_le_bytes());
        archive_bytes.extend_from_slice(&pal_body);

        let vin_offset = archive_bytes.len() as u32;
        archive_bytes.extend_from_slice(&name_field("FILES.VIN"));
        archive_bytes.extend_from_slice(&4u32.to_le_bytes());
        archive_bytes.extend_from_slice(b"junk");

        // Build RESOURCE.MAP referencing both.
        let mut map = Vec::new();
        map.extend_from_slice(&[0, 0, 0, 2, 0, 0]); // 6 unknown bytes
        map.extend_from_slice(&name_field("RESOURCE.001"));
        map.extend_from_slice(&2u16.to_le_bytes());
        map.extend_from_slice(&(pal_body.len() as u32).to_le_bytes());
        map.extend_from_slice(&pal_offset.to_le_bytes());
        map.extend_from_slice(&4u32.to_le_bytes());
        map.extend_from_slice(&vin_offset.to_le_bytes());

        let archive = Archive::parse(&map, &archive_bytes).unwrap();
        assert_eq!(archive.data_file_name, "RESOURCE.001");
        assert_eq!(archive.palettes.len(), 1);
        assert_eq!(archive.skipped, vec!["FILES.VIN".to_string()]);
        assert_eq!(archive.palette().unwrap().rgb(0), [63 << 2, 0, 0]);
    }
}
