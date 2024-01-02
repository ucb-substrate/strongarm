use crate::ClockedDiffComparatorIo;
use atoll::route::GreedyBfsRouter;
use atoll::{IoBuilder, Tile, TileBuilder};
use serde::{Deserialize, Serialize};
use sky130pdk::atoll::{MosLength, NmosTile, PmosTile, Sky130ViaMaker};
use sky130pdk::Sky130Pdk;
use substrate::block::Block;
use substrate::geometry::align::AlignMode;
use substrate::geometry::rect::Rect;
use substrate::io::layout::{Builder, IoShape};
use substrate::io::schematic::Bundle;
use substrate::io::{InOut, Input, Io, MosIo, MosIoSchematic, Signal};
use substrate::layout::{ExportsLayoutData, Layout};
use substrate::schematic::{CellBuilder, ExportsNestedData, Schematic};

#[derive(Debug, Default, Clone, Io)]
pub struct TwoFingerMosTileIo {
    pub sd0: InOut<Signal>,
    pub sd1: InOut<Signal>,
    pub sd2: InOut<Signal>,
    pub g: Input<Signal>,
    pub b: InOut<Signal>,
}

impl From<MosIoSchematic> for TwoFingerMosTileIoSchematic {
    fn from(value: Bundle<MosIo>) -> Self {
        Self {
            sd0: value.s,
            sd1: value.d,
            sd2: value.s,
            g: value.g,
            b: value.b,
        }
    }
}

#[derive(Serialize, Deserialize, Block, Clone, Debug, Hash, PartialEq, Eq)]
#[substrate(io = "TwoFingerMosTileIo")]
struct TwoFingerMosTile {
    w: i64,
    l: MosLength,
    kind: MosTileKind,
}

impl TwoFingerMosTile {
    pub fn new(w: i64, l: MosLength, kind: MosTileKind) -> Self {
        Self { w, l, kind }
    }

    pub fn pmos(w: i64, l: MosLength) -> Self {
        Self::new(w, l, MosTileKind::Pmos)
    }

    pub fn nmos(w: i64, l: MosLength) -> Self {
        Self::new(w, l, MosTileKind::Nmos)
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum MosTileKind {
    Pmos,
    Nmos,
}

impl ExportsNestedData for TwoFingerMosTile {
    type NestedData = ();
}

impl Schematic<Sky130Pdk> for TwoFingerMosTile {
    fn schematic(
        &self,
        io: &Bundle<<Self as Block>::Io>,
        cell: &mut CellBuilder<Sky130Pdk>,
    ) -> substrate::error::Result<Self::NestedData> {
        cell.flatten();
        match self.kind {
            MosTileKind::Pmos => {
                let pmos = cell.instantiate(PmosTile::new(self.w, self.l, 2));
                cell.connect(pmos.io().g, io.g);
                cell.connect(pmos.io().b, io.b);
                cell.connect(pmos.io().sd[0], io.sd0);
                cell.connect(pmos.io().sd[1], io.sd1);
                cell.connect(pmos.io().sd[2], io.sd2);
            }
            MosTileKind::Nmos => {
                let nmos = cell.instantiate(NmosTile::new(self.w, self.l, 2));
                cell.connect(nmos.io().g, io.g);
                cell.connect(nmos.io().b, io.b);
                cell.connect(nmos.io().sd[0], io.sd0);
                cell.connect(nmos.io().sd[1], io.sd1);
                cell.connect(nmos.io().sd[2], io.sd2);
            }
        }
        Ok(())
    }
}

impl ExportsLayoutData for TwoFingerMosTile {
    type LayoutData = ();
}

impl Layout<Sky130Pdk> for TwoFingerMosTile {
    fn layout(&self, io: &mut Builder<<Self as Block>::Io>, cell: &mut substrate::layout::CellBuilder<Sky130Pdk>) -> substrate::error::Result<Self::LayoutData> {
        match self.kind {
            MosTileKind::Pmos => {
                let pmos = cell.generate(PmosTile::new(self.w, self.l, 2));
                io.g.merge(pmos.io().g);
                io.sd0.merge(pmos.io().sd[0].clone());
                io.sd1.merge(pmos.io().sd[1].clone());
                io.sd2.merge(pmos.io().sd[2].clone());
                io.b.merge(pmos.io().b);
                cell.draw(pmos)?;
            }
            MosTileKind::Nmos => {
                let nmos = cell.generate(NmosTile::new(self.w, self.l, 2));
                io.g.merge(nmos.io().g);
                io.sd0.merge(nmos.io().sd[0].clone());
                io.sd1.merge(nmos.io().sd[1].clone());
                io.sd2.merge(nmos.io().sd[2].clone());
                io.b.merge(nmos.io().b);
                cell.draw(nmos)?;
            }
        }
        Ok(())
    }
}

#[derive(Serialize, Deserialize, Block, Copy, Clone, Debug, Hash, PartialEq, Eq)]
#[substrate(io = "ClockedDiffComparatorIo")]
pub struct AtollStrongArmInstance {
    pub half_tail_w: i64,
    pub input_pair_w: i64,
    pub inv_nmos_w: i64,
    pub inv_pmos_w: i64,
    pub precharge_w: i64,
}

impl ExportsNestedData for AtollStrongArmInstance {
    type NestedData = ();
}

impl ExportsLayoutData for AtollStrongArmInstance {
    type LayoutData = ();
}

impl Tile<Sky130Pdk> for AtollStrongArmInstance {
    fn tile<'a>(
        &self,
        io: IoBuilder<'a, Self>,
        cell: &mut TileBuilder<'a, Sky130Pdk>,
    ) -> substrate::error::Result<(
        <Self as ExportsNestedData>::NestedData,
        <Self as ExportsLayoutData>::LayoutData,
    )> {
        let mut ltail =
            cell.generate_primitive(TwoFingerMosTile::nmos(self.half_tail_w, MosLength::L150));
        let mut rtail =
            cell.generate_primitive(TwoFingerMosTile::nmos(self.half_tail_w, MosLength::L150));
        let mut linput =
            cell.generate_primitive(TwoFingerMosTile::nmos(self.input_pair_w, MosLength::L150));
        let mut rinput =
            cell.generate_primitive(TwoFingerMosTile::nmos(self.input_pair_w, MosLength::L150));
        let mut linvn =
            cell.generate_primitive(TwoFingerMosTile::nmos(self.inv_nmos_w, MosLength::L150));
        let mut rinvn =
            cell.generate_primitive(TwoFingerMosTile::nmos(self.inv_nmos_w, MosLength::L150));
        let mut linvp =
            cell.generate_primitive(TwoFingerMosTile::pmos(self.inv_pmos_w, MosLength::L150));
        let mut rinvp =
            cell.generate_primitive(TwoFingerMosTile::pmos(self.inv_pmos_w, MosLength::L150));
        let mut lprecharge1 =
            cell.generate_primitive(TwoFingerMosTile::pmos(self.precharge_w, MosLength::L150));
        let mut rprecharge1 =
            cell.generate_primitive(TwoFingerMosTile::pmos(self.precharge_w, MosLength::L150));
        let mut lprecharge2 =
            cell.generate_primitive(TwoFingerMosTile::pmos(self.precharge_w, MosLength::L150));
        let mut rprecharge2 =
            cell.generate_primitive(TwoFingerMosTile::pmos(self.precharge_w, MosLength::L150));

        let mut prev = None;

        for (l, r) in [
            (&mut ltail, &mut rtail),
            (&mut linput, &mut rinput),
            (&mut linvn, &mut rinvn),
        ] {
            if let Some(prev) = prev {
                l.align_rect_mut(prev, AlignMode::Left, 0);
                l.align_rect_mut(prev, AlignMode::Beneath, 0);
            }

            r.align_mut(l, AlignMode::Bottom, 0);
            r.align_mut(l, AlignMode::ToTheRight, 0);

            prev = Some(l.lcm_bounds());
        }

        for (l, r) in [
            (&mut linvp, &mut rinvp),
            (&mut lprecharge1, &mut rprecharge1),
            (&mut lprecharge2, &mut rprecharge2),
        ] {
            l.align_rect_mut(prev.unwrap(), AlignMode::Left, 0);
            l.align_rect_mut(prev.unwrap(), AlignMode::Beneath, 0);
            r.align_mut(l, AlignMode::Bottom, 0);
            r.align_mut(l, AlignMode::ToTheRight, 0);

            prev = Some(l.lcm_bounds());
        }

        let tail = cell.signal("tail", Signal);
        let intn = cell.signal("intn", Signal);
        let intp = cell.signal("intp", Signal);

        for inst in [&ltail, &rtail] {
            cell.connect(
                inst.io(),
                Bundle::<TwoFingerMosTileIo>::from(Bundle::<MosIo> {
                    d: tail,
                    g: io.schematic.clock,
                    s: io.schematic.vss,
                    b: io.schematic.vss,
                }),
            );
        }

        cell.connect(
            linput.io(),
            Bundle::<TwoFingerMosTileIo>::from(Bundle::<MosIo> {
                d: intn,
                g: io.schematic.input.p,
                s: tail,
                b: io.schematic.vss,
            }),
        );
        cell.connect(
            rinput.io(),
                Bundle::<TwoFingerMosTileIo>::from(Bundle::<MosIo> {
                d: intp,
                g: io.schematic.input.n,
                s: tail,
                b: io.schematic.vss,
            }),
        );

        cell.connect(
            linvn.io(),
                Bundle::<TwoFingerMosTileIo>::from(Bundle::<MosIo> {
                d: io.schematic.output.n,
                g: io.schematic.output.p,
                s: intn,
                b: io.schematic.vss,
            }),
        );

        cell.connect(
            rinvn.io(),
                Bundle::<TwoFingerMosTileIo>::from(Bundle::<MosIo> {
                d: io.schematic.output.p,
                g: io.schematic.output.n,
                s: intp,
                b: io.schematic.vss,
            }),
        );

        cell.connect(
            linvp.io(),
                Bundle::<TwoFingerMosTileIo>::from(Bundle::<MosIo> {
                d: io.schematic.output.n,
                g: io.schematic.output.p,
                s: io.schematic.vdd,
                b: io.schematic.vdd,
            }),
        );
        cell.connect(
            rinvp.io(),
                Bundle::<TwoFingerMosTileIo>::from(Bundle::<MosIo> {
                d: io.schematic.output.p,
                g: io.schematic.output.n,
                s: io.schematic.vdd,
                b: io.schematic.vdd,
            }),
        );

        cell.connect(
            lprecharge1.io(),
                Bundle::<TwoFingerMosTileIo>::from(Bundle::<MosIo> {
                d: io.schematic.output.n,
                g: io.schematic.clock,
                s: io.schematic.vdd,
                b: io.schematic.vdd,
            }),
        );
        cell.connect(
            rprecharge1.io(),
                Bundle::<TwoFingerMosTileIo>::from(Bundle::<MosIo> {
                d: io.schematic.output.p,
                g: io.schematic.clock,
                s: io.schematic.vdd,
                b: io.schematic.vdd,
            }),
        );
        cell.connect(
            lprecharge2.io(),
                Bundle::<TwoFingerMosTileIo>::from(Bundle::<MosIo> {
                d: intn,
                g: io.schematic.clock,
                s: io.schematic.vdd,
                b: io.schematic.vdd,
            }),
        );
        cell.connect(
            rprecharge2.io(),
                Bundle::<TwoFingerMosTileIo>::from(Bundle::<MosIo> {
                d: intp,
                g: io.schematic.clock,
                s: io.schematic.vdd,
                b: io.schematic.vdd,
            }),
        );

        let (_, ltail) = cell.draw(ltail)?;
        cell.draw(rtail)?;
        let (_, linput) = cell.draw(linput)?;
        let (_, rinput) = cell.draw(rinput)?;
        cell.draw(linvn)?;
        cell.draw(rinvn)?;
        let (_, linvp) = cell.draw(linvp)?;
        let (_, rinvp) = cell.draw(rinvp)?;
        cell.draw(lprecharge1)?;
        cell.draw(rprecharge1)?;
        cell.draw(lprecharge2)?;
        cell.draw(rprecharge2)?;

        cell.set_top_layer(2);
        cell.set_router(GreedyBfsRouter);
        cell.set_via_maker(Sky130ViaMaker);

        // todo: add correct port geometry
        let tmp = IoShape::with_layers(
            cell.ctx().layers.li1,
            Rect::from_sides(-500, -500, -400, -400),
        );

        io.layout.clock.set_primary(ltail.io().g.primary);
        io.layout.vdd.push(linvp.io().b.primary);
        io.layout.vdd.set_primary(linvp.io().sd0.primary);
        io.layout.vss.push(ltail.io().b.primary);
        io.layout.vss.set_primary(ltail.io().sd0.primary);
        io.layout.input.p.set_primary(linput.io().g.primary);
        io.layout.input.n.set_primary(rinput.io().g.primary);
        io.layout.output.p.set_primary(linvp.io().g.primary);
        io.layout.output.n.set_primary(rinvp.io().g.primary);

        Ok(((), ()))
    }
}
