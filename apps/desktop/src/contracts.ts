// All wire types are generated from Rust; only UI defaults belong here.
export type * from "./generated";
import type { PlateParameters } from "./generated";
export const defaultPlate: PlateParameters = {
  schema_version: 1,
  width: 200,
  height: 120,
  hole_diameter: 10,
  edge_offset: 20,
  origin: [0, 0],
  units: "mm",
  add_dimensions: true,
};
