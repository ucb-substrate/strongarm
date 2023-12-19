use approx::abs_diff_eq;
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use sky130pdk::corner::Sky130Corner;
use sky130pdk::mos::MosParams;
use sky130pdk::mos::{Nfet01v8, Pfet01v8};
use sky130pdk::Sky130Pdk;
use spectre::blocks::{Pulse, Vsource};
use spectre::tran::Tran;
use spectre::{ErrPreset, Spectre};
use substrate::block::Block;
use substrate::context::{Context, PdkContext};
use substrate::io::TestbenchIo;
use substrate::io::{
    DiffPair, DiffPairSchematic, InOut, Input, Io, MosIoSchematic, Node, Output, SchematicType,
    Signal,
};
use substrate::pdk::corner::Pvt;
use substrate::schematic::{Cell, CellBuilder, ExportsNestedData, NestedData, Schematic};
use substrate::simulation::{SimController, SimulationContext, Simulator, Testbench};

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

impl ExportsNestedData for StrongArmInstance {
    type NestedData = ();
}

impl Schematic<Sky130Pdk> for StrongArmInstance {
    fn schematic(
        &self,
        io: &<<Self as Block>::Io as SchematicType>::Bundle,
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

#[derive(Serialize, Deserialize, Block, Clone, Debug, Hash, PartialEq, Eq)]
#[substrate(io = "TestbenchIo")]
pub struct StrongArmTranTb {
    dut: StrongArmInstance,
    vinp: Decimal,
    vinn: Decimal,
    pvt: Pvt<Sky130Corner>,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, NestedData)]
pub struct StrongArmTranTbNodes {
    vop: Node,
    von: Node,
    vinn: Node,
    vinp: Node,
    clk: Node,
}
impl ExportsNestedData for StrongArmTranTb {
    type NestedData = StrongArmTranTbNodes;
}

impl Schematic<Spectre> for StrongArmTranTb {
    fn schematic(
        &self,
        io: &<<Self as Block>::Io as SchematicType>::Bundle,
        cell: &mut CellBuilder<Spectre>,
    ) -> substrate::error::Result<Self::NestedData> {
        let dut = cell
            .sub_builder::<Sky130Pdk>()
            .instantiate(self.dut.clone());

        let vinp = cell.instantiate(Vsource::dc(self.vinp));
        let vinn = cell.instantiate(Vsource::dc(self.vinn));
        let vdd = cell.instantiate(Vsource::dc(self.pvt.voltage));
        let vclk = cell.instantiate(Vsource::pulse(Pulse {
            val0: dec!(0),
            val1: self.pvt.voltage,
            period: Some(dec!(1000)),
            width: Some(dec!(100)),
            delay: Some(dec!(10e-9)),
            rise: Some(dec!(100e-12)),
            fall: Some(dec!(100e-12)),
        }));

        cell.connect(io.vss, vinp.io().n);
        cell.connect(io.vss, vinn.io().n);
        cell.connect(io.vss, vdd.io().n);
        cell.connect(io.vss, vclk.io().n);

        let output = cell.signal("output", DiffPair::default());

        cell.connect(
            ClockedDiffComparatorIoSchematic {
                input: DiffPairSchematic {
                    p: *vinp.io().p,
                    n: *vinn.io().n,
                },
                output: output.clone(),
                clock: *vclk.io().p,
                vdd: *vdd.io().p,
                vss: io.vss,
            },
            dut.io(),
        );

        Ok(StrongArmTranTbNodes {
            vop: output.p,
            von: output.n,
            vinn: *vinn.io().p,
            vinp: *vinp.io().p,
            clk: *vclk.io().p,
        })
    }
}

use substrate::simulation::data::{tran, FromSaved, Save, SaveTb};

#[derive(Debug, Clone, Serialize, Deserialize, FromSaved)]
pub struct ComparatorSim {
    t: tran::Time,
    vop: tran::Voltage,
    von: tran::Voltage,
    vinn: tran::Voltage,
    vinp: tran::Voltage,
    clk: tran::Voltage,
}

/// The decision made by a comparator.
#[derive(Copy, Clone, Hash, Eq, PartialEq, Ord, PartialOrd, Debug, Serialize, Deserialize)]
pub enum ComparatorDecision {
    /// Negative.
    ///
    /// The negative input was larger than the positive input.
    Neg,
    /// Positive.
    ///
    /// The positive input was larger than the negative input.
    Pos,
}

impl SaveTb<Spectre, Tran, ComparatorSim> for StrongArmTranTb {
    fn save_tb(
        ctx: &SimulationContext<Spectre>,
        cell: &Cell<Self>,
        opts: &mut <Spectre as Simulator>::Options,
    ) -> <ComparatorSim as FromSaved<Spectre, Tran>>::SavedKey {
        ComparatorSimSavedKey {
            t: tran::Time::save(ctx, (), opts),
            vop: tran::Voltage::save(ctx, cell.data().vop, opts),
            von: tran::Voltage::save(ctx, cell.data().von, opts),
            vinn: tran::Voltage::save(ctx, cell.data().vinn, opts),
            vinp: tran::Voltage::save(ctx, cell.data().vinp, opts),
            clk: tran::Voltage::save(ctx, cell.data().clk, opts),
        }
    }
}

impl Testbench<Spectre> for StrongArmTranTb {
    type Output = Option<ComparatorDecision>;

    fn run(&self, sim: SimController<Spectre, Self>) -> Self::Output {
        let mut opts = spectre::Options::default();
        sim.set_option(self.pvt.corner, &mut opts);
        let wav: ComparatorSim = sim
            .simulate(
                opts,
                Tran {
                    stop: dec!(20e-9),
                    start: None,
                    errpreset: Some(ErrPreset::Conservative),
                },
            )
            .expect("failed to run simulation");

        let von = *wav.von.last().unwrap();
        let vop = *wav.vop.last().unwrap();

        let vdd = self.pvt.voltage.to_f64().unwrap();
        if abs_diff_eq!(von, 0.0, epsilon = 1e-6) && abs_diff_eq!(vop, vdd, epsilon = 1e-6) {
            Some(ComparatorDecision::Pos)
        } else if abs_diff_eq!(von, vdd, epsilon = 1e-6) && abs_diff_eq!(vop, 0.0, epsilon = 1e-6) {
            Some(ComparatorDecision::Neg)
        } else {
            None
        }
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

    #[test]
    fn sim_strongarm() {
        let work_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/sim_strongarm");
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
}
