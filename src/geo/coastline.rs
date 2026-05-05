//! Loads embedded Natural Earth coastline GeoJSON (public domain) into a
//! flat list of polylines for fast painting.
//!
//! The asset is `assets/coastline.geojson`, included at compile time.

use crate::geo::LatLon;
use anyhow::{Context, Result};
use geojson::{GeoJson, Geometry, GeometryValue, Position};

const RAW: &str = include_str!("../../assets/coastline.geojson");

/// All coastline polylines, in (lat, lon) degrees.
pub struct Coastline {
    pub lines: Vec<Vec<LatLon>>,
}

impl Coastline {
    pub fn load() -> Result<Self> {
        let gj: GeoJson = RAW.parse().context("parse coastline geojson")?;
        let mut lines = Vec::new();
        match gj {
            GeoJson::FeatureCollection(fc) => {
                for f in fc.features {
                    if let Some(g) = f.geometry {
                        push_geometry(&g, &mut lines);
                    }
                }
            }
            GeoJson::Feature(f) => {
                if let Some(g) = f.geometry {
                    push_geometry(&g, &mut lines);
                }
            }
            GeoJson::Geometry(g) => push_geometry(&g, &mut lines),
        }
        Ok(Self { lines })
    }
}

fn push_geometry(g: &Geometry, out: &mut Vec<Vec<LatLon>>) {
    match &g.value {
        GeometryValue::LineString { coordinates } => out.push(coords_to_latlons(coordinates)),
        GeometryValue::MultiLineString { coordinates } => {
            for ls in coordinates {
                out.push(coords_to_latlons(ls));
            }
        }
        GeometryValue::Polygon { coordinates } => {
            for ring in coordinates {
                out.push(coords_to_latlons(ring));
            }
        }
        GeometryValue::MultiPolygon { coordinates } => {
            for rings in coordinates {
                for ring in rings {
                    out.push(coords_to_latlons(ring));
                }
            }
        }
        GeometryValue::GeometryCollection { geometries } => {
            for sub in geometries {
                push_geometry(sub, out);
            }
        }
        _ => {}
    }
}

fn coords_to_latlons(coords: &[Position]) -> Vec<LatLon> {
    coords
        .iter()
        .filter_map(|c| {
            if c.len() >= 2 {
                Some(LatLon {
                    lat: c[1] as f32,
                    lon: c[0] as f32,
                })
            } else {
                None
            }
        })
        .collect()
}
