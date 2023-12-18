use serde::{Deserialize, Serialize};
use sky130pdk::mos::{Nfet01v8, Pfet01v8};
use substrate::block::Block;
use substrate::io::{DiffPair, InOut, Input, Io, MosIo, MosIoSchematic, Output, SchematicType, Signal};
use sky130pdk::mos::MosParams;
use sky130pdk::Sky130Pdk;
use substrate::schematic::{CellBuilder, ExportsNestedData, Schematic};


#[derive(Debug, Default, Clone, Io)]
pub struct ClockedDiffComparatorIo {
    pub input: Input<DiffPair>,
    pub output: Output<DiffPair>,
    pub clock: Input<Signal>,
    pub vdd: InOut<Signal>,
    pub vss: InOut<Signal>,
}
#[derive(Serialize, Deserialize, Block, Clone, Debug, Hash, PartialEq, Eq)]
#[substrate(io = "ClockedDiffComparatorIo")]
pub struct StrongArmInstance {
    tail: MosParams,
    input_pair: MosParams,
    inv_nmos: MosParams,
    inv_pmos: MosParams,
    precharge: MosParams,
}

impl ExportsNestedData for StrongArmInstance { type NestedData = (); }

impl Schematic<Sky130Pdk> for StrongArmInstance {
    fn schematic(&self, io: &<<Self as Block>::Io as SchematicType>::Bundle, cell: &mut CellBuilder<Sky130Pdk>) -> substrate::error::Result<Self::NestedData> {
        let tail = cell.signal("tail", Signal);
        let intn = cell.signal("intn", Signal);
        let intp = cell.signal("intp", Signal);

        cell.instantiate_connected(Nfet01v8::new(self.tail), MosIoSchematic {
            d: tail,
            g: io.clock,
            s: io.vss,
            b: io.vss,
        });

        cell.instantiate_connected(Nfet01v8::new(self.input_pair), MosIoSchematic {
            d: intn,
            g: io.input.p,
            s: tail,
            b: io.vss,
        });
        cell.instantiate_connected(Nfet01v8::new(self.input_pair), MosIoSchematic {
            d: intp,
            g: io.input.n,
            s: tail,
            b: io.vss,
        });

        cell.instantiate_connected(Nfet01v8::new(self.inv_nmos), MosIoSchematic {
            d: io.output.n,
            g: io.output.p,
            s: intn,
            b: io.vss,
        });
        cell.instantiate_connected(Nfet01v8::new(self.inv_nmos), MosIoSchematic {
            d: io.output.p,
            g: io.output.n,
            s: intp,
            b: io.vss,
        });

        cell.instantiate_connected(Pfet01v8::new(self.inv_pmos), MosIoSchematic {
            d: io.output.n,
            g: io.output.p,
            s: io.vdd,
            b: io.vdd,
        });
        cell.instantiate_connected(Pfet01v8::new(self.inv_pmos), MosIoSchematic {
            d: io.output.p,
            g: io.output.n,
            s: io.vdd,
            b: io.vdd,
        });

        cell.instantiate_connected(Pfet01v8::new(self.precharge), MosIoSchematic {
            d: io.output.n,
            g: io.clock,
            s: io.vdd,
            b: io.vdd,
        });
        cell.instantiate_connected(Pfet01v8::new(self.precharge), MosIoSchematic {
            d: io.output.p,
            g: io.clock,
            s: io.vdd,
            b: io.vdd,
        });
        cell.instantiate_connected(Pfet01v8::new(self.precharge), MosIoSchematic {
            d: intn,
            g: io.clock,
            s: io.vdd,
            b: io.vdd,
        });
        cell.instantiate_connected(Pfet01v8::new(self.precharge), MosIoSchematic {
            d: intp,
            g: io.clock,
            s: io.vdd,
            b: io.vdd,
        });

        Ok(())
    }
}
