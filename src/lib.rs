use approx::abs_diff_eq;
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use sky130pdk::corner::Sky130Corner;
use sky130pdk::mos::MosParams;
use sky130pdk::mos::{Nfet01v8, Pfet01v8};
use sky130pdk::Sky130Pdk;
use spectre::Spectre;
use substrate::block::Block;
use substrate::context::{Context, PdkContext};
use substrate::io::schematic::HardwareType;
use substrate::io::{DiffPair, InOut, Input, Io, MosIoSchematic, Output, Signal};
use substrate::pdk::corner::Pvt;
use substrate::schematic::{CellBuilder, ExportsNestedData, NestedData, Schematic};
use substrate::simulation::{Simulator, Testbench};

pub mod atoll;
pub mod tb;

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
    pub tail: MosParams,
    pub input_pair: MosParams,
    pub inv_nmos: MosParams,
    pub inv_pmos: MosParams,
    pub precharge: MosParams,
}

impl ExportsNestedData for StrongArmInstance {
    type NestedData = ();
}

impl Schematic<Sky130Pdk> for StrongArmInstance {
    fn schematic(
        &self,
        io: &<<Self as Block>::Io as HardwareType>::Bundle,
        cell: &mut CellBuilder<Sky130Pdk>,
    ) -> substrate::error::Result<Self::NestedData> {
        let tail = cell.signal("tail", Signal);
        let intn = cell.signal("intn", Signal);
        let intp = cell.signal("intp", Signal);

        cell.instantiate_connected(
            Nfet01v8::new(self.tail),
            MosIoSchematic {
                d: tail,
                g: io.clock,
                s: io.vss,
                b: io.vss,
            },
        );

        cell.instantiate_connected(
            Nfet01v8::new(self.input_pair),
            MosIoSchematic {
                d: intn,
                g: io.input.p,
                s: tail,
                b: io.vss,
            },
        );
        cell.instantiate_connected(
            Nfet01v8::new(self.input_pair),
            MosIoSchematic {
                d: intp,
                g: io.input.n,
                s: tail,
                b: io.vss,
            },
        );

        cell.instantiate_connected(
            Nfet01v8::new(self.inv_nmos),
            MosIoSchematic {
                d: io.output.n,
                g: io.output.p,
                s: intn,
                b: io.vss,
            },
        );
        cell.instantiate_connected(
            Nfet01v8::new(self.inv_nmos),
            MosIoSchematic {
                d: io.output.p,
                g: io.output.n,
                s: intp,
                b: io.vss,
            },
        );

        cell.instantiate_connected(
            Pfet01v8::new(self.inv_pmos),
            MosIoSchematic {
                d: io.output.n,
                g: io.output.p,
                s: io.vdd,
                b: io.vdd,
            },
        );
        cell.instantiate_connected(
            Pfet01v8::new(self.inv_pmos),
            MosIoSchematic {
                d: io.output.p,
                g: io.output.n,
                s: io.vdd,
                b: io.vdd,
            },
        );

        cell.instantiate_connected(
            Pfet01v8::new(self.precharge),
            MosIoSchematic {
                d: io.output.n,
                g: io.clock,
                s: io.vdd,
                b: io.vdd,
            },
        );
        cell.instantiate_connected(
            Pfet01v8::new(self.precharge),
            MosIoSchematic {
                d: io.output.p,
                g: io.clock,
                s: io.vdd,
                b: io.vdd,
            },
        );
        cell.instantiate_connected(
            Pfet01v8::new(self.precharge),
            MosIoSchematic {
                d: intn,
                g: io.clock,
                s: io.vdd,
                b: io.vdd,
            },
        );
        cell.instantiate_connected(
            Pfet01v8::new(self.precharge),
            MosIoSchematic {
                d: intp,
                g: io.clock,
                s: io.vdd,
                b: io.vdd,
            },
        );

        Ok(())
    }
}

pub fn sky130_ctx() -> PdkContext<Sky130Pdk> {
    let pdk_root = std::env::var("SKY130_COMMERCIAL_PDK_ROOT")
        .expect("the SKY130_COMMERCIAL_PDK_ROOT environment variable must be set");
    Context::builder()
        .install(Spectre::default())
        .install(Sky130Pdk::commercial(pdk_root))
        .build()
        .with_pdk()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::atoll::AtollStrongArmInstance;
    use crate::tb::{ComparatorDecision, StrongArmTranTb};
    use ::atoll::TileWrapper;
    use sky130pdk::atoll::{MosLength, NmosTile, PmosTile};
    use std::path::PathBuf;

    #[test]
    fn sim_strongarm() {
        let work_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/build/sim_strongarm");
        let tb = StrongArmTranTb {
            dut: StrongArmInstance {
                tail: MosParams {
                    w: 5_000,
                    l: 150,
                    nf: 1,
                },
                input_pair: MosParams {
                    w: 8_000,
                    l: 150,
                    nf: 1,
                },
                inv_nmos: MosParams {
                    w: 4_000,
                    l: 150,
                    nf: 1,
                },
                inv_pmos: MosParams {
                    w: 2_000,
                    l: 150,
                    nf: 1,
                },
                precharge: MosParams {
                    w: 2_000,
                    l: 150,
                    nf: 1,
                },
            },
            vinp: dec!(0.8),
            vinn: dec!(0.6),
            pvt: Pvt {
                corner: Sky130Corner::Tt,
                voltage: dec!(1.8),
                temp: dec!(25.0),
            },
        };
        let ctx = sky130_ctx();
        let decision = ctx
            .simulate(tb, work_dir)
            .expect("failed to run simulation")
            .expect("comparator output did not rail");
        assert_eq!(
            decision,
            ComparatorDecision::Pos,
            "comparator produced incorrect decision"
        );
    }

    #[test]
    fn layout_strongarm() {
        let work_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/build/layout_strongarm");
        let gds_path = PathBuf::from(work_dir).join("layout.gds");
        let ctx = sky130_ctx();

        ctx.write_layout(
            TileWrapper::new(AtollStrongArmInstance {
                half_tail_w: 1_250,
                input_pair_w: 4_000,
                inv_nmos_w: 2_000,
                inv_pmos_w: 1_000,
                precharge_w: 1_000,
            }),
            gds_path,
        )
        .expect("failed to write layout");
    }
}
