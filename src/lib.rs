use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use sky130pdk::corner::Sky130Corner;
use sky130pdk::mos::MosParams;
use sky130pdk::mos::{Nfet01v8, Pfet01v8};
use sky130pdk::Sky130Pdk;
use spectre::blocks::Vsource;
use spectre::tran::Tran;
use spectre::Spectre;
use substrate::block::Block;
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
        let vclk = cell.instantiate(Vsource::dc(self.pvt.voltage));

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
    type Output = ComparatorSim;

    fn run(&self, sim: SimController<Spectre, Self>) -> Self::Output {
        let mut opts = spectre::Options::default();
        sim.set_option(self.pvt.corner, &mut opts);
        sim.simulate(
            opts,
            Tran {
                stop: dec!(2e-9),
                start: None,
                errpreset: None,
            },
        )
        .expect("failed to run simulation")
    }
}
