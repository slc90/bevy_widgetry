"""Reproduce the 80px Kamuri collection and its shared Rust asset catalogue.

Run with: uv run --python 3.13 --with pillow --with numpy python <this file>
"""

import argparse
from dataclasses import dataclass
import hashlib
import html
import json
import math
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw


ROOT = Path(__file__).resolve().parents[3]
CELL, SCALE, COLS = 80, 4, 8
TAU = math.tau
INK = "#343873"
WHITE = "#FAF6FF"
BLUE = "#4789FF"
CYAN = "#81DFFF"
LILAC = "#B7A1F4"
RED = "#F16A91"


@dataclass(frozen=True)
class Spec:
    name: str
    frames: int
    millis: int
    hotspot: tuple[int, int]
    motion: str


# Each timing is selected for the gesture, rather than inherited from Pointer.
SPECS = [
    Spec("Default", 32, 80, (10, 12), "A ribbon flows down a stationary arrow; the point stays still."),
    Spec("Pointer", 28, 70, (20, 34), "A fingertip rests on a button while a soft press wave expands."),
    Spec("ContextMenu", 36, 80, (10, 12), "Menu rows are explored in order beside a fixed arrow."),
    Spec("Help", 40, 80, (10, 12), "A question mark traces a small inquisitive nod; its dot stays put."),
    Spec("Text", 32, 90, (17, 47), "An ink bead travels vertically along a steady insertion beam."),
    Spec("VerticalText", 30, 90, (28, 54), "An ink bead writes left to right along a horizontal beam."),
    Spec("Cell", 32, 80, (26, 54), "Four neighbouring grid cells are visited without moving the active cell."),
    Spec("Crosshair", 40, 60, (27, 53), "Separated calibration arcs rotate around a fixed fine cross."),
    Spec("Grab", 32, 80, (26, 51), "Open fingers invite a grasp, with a relaxed wrist sway."),
    Spec("Grabbing", 28, 60, (26, 53), "A closed grip compresses a held capsule; the grip centre is fixed."),
    Spec("Move", 32, 60, (29, 53), "Four cargo beads depart along the four move axes."),
    Spec("Copy", 36, 70, (24, 54), "A second document peels away and settles behind the source."),
    Spec("Alias", 40, 65, (24, 55), "A route bead follows a bent shortcut arrow."),
    Spec("NoDrop", 32, 75, (25, 54), "A parcel approaches a barrier and gently recoils."),
    Spec("EResize", 28, 65, (28, 54), "An eastbound shuttle travels inside fixed resize endpoints."),
    Spec("NResize", 30, 60, (28, 54), "A northbound shuttle travels inside fixed resize endpoints."),
    Spec("NeResize", 32, 60, (28, 54), "A northeast shuttle follows the diagonal resize axis."),
    Spec("NwResize", 34, 60, (28, 54), "A northwest shuttle follows the diagonal resize axis."),
    Spec("SResize", 28, 70, (28, 54), "A southbound shuttle travels inside fixed resize endpoints."),
    Spec("SeResize", 30, 70, (28, 54), "A southeast shuttle follows the diagonal resize axis."),
    Spec("SwResize", 32, 70, (28, 54), "A southwest shuttle follows the diagonal resize axis."),
    Spec("WResize", 34, 65, (28, 54), "A westbound shuttle travels inside fixed resize endpoints."),
    Spec("EwResize", 36, 60, (28, 54), "Two shuttles breathe outward along the horizontal axis."),
    Spec("NsResize", 32, 65, (28, 54), "Two shuttles breathe outward along the vertical axis."),
    Spec("NeswResize", 36, 65, (28, 54), "Opposed shuttles separate along the northeast diagonal."),
    Spec("NwseResize", 40, 60, (28, 54), "Opposed shuttles separate along the southeast diagonal."),
    Spec("ColResize", 32, 75, (28, 54), "Column rails remain anchored while side handles slide outward."),
    Spec("RowResize", 36, 70, (28, 54), "Row rails remain anchored while upper and lower handles slide outward."),
    Spec("AllScroll", 48, 50, (29, 53), "A compass bead visits all four scroll directions clockwise."),
    Spec("ZoomIn", 32, 75, (25, 52), "An inner lens ring grows beneath a fixed plus and lens centre."),
    Spec("ZoomOut", 36, 75, (25, 52), "An inner lens ring contracts beneath a fixed minus and lens centre."),
    Spec("Progress", 40, 55, (10, 12), "An arrow remains usable while three task beads chase around a dial."),
    Spec("Wait", 48, 65, (25, 56), "Kamuri scoops pudding while the fixed-centre waiting ring turns."),
    Spec("NotAllowed", 40, 85, (25, 54), "A perimeter sentinel guards a stationary prohibition slash."),
    Spec("AddSelection", 32, 70, (26, 54), "An extra tile joins a selection beside a fixed plus badge."),
    Spec("RemoveSelection", 36, 70, (26, 54), "A tile withdraws from a selection beside a fixed minus badge."),
    Spec("RangeSelection", 40, 65, (16, 56), "A span extends from a fixed origin and returns along its ruler."),
    Spec("DragSort", 36, 60, (26, 54), "Two list rows exchange places around a fixed grip."),
    Spec("InsertDrop", 32, 80, (25, 56), "A card descends into a gap above a stationary insertion bar."),
    Spec("ReplaceDrop", 40, 70, (25, 54), "An incoming card cross-slides over an existing card."),
    Spec("HorizontalValueAdjust", 36, 60, (28, 54), "A tiny scale sweeps under a stationary horizontal adjustment handle."),
    Spec("VerticalValueAdjust", 40, 60, (28, 54), "A tiny scale sweeps under a stationary vertical adjustment handle."),
    Spec("PanReady", 40, 70, (25, 54), "A relaxed palm awaits a pan; a map contour circles behind it."),
    Spec("Panning", 32, 55, (25, 54), "A gripped map flows beneath the fixed panning anchor."),
    Spec("ReadOnly", 48, 80, (25, 56), "A light scans a locked page; the closed shackle and keyhole never move."),
]


class Canvas:
    def __init__(self):
        self.image = Image.new("RGBA", (CELL * SCALE, CELL * SCALE))
        self.d = ImageDraw.Draw(self.image)

    def points(self, points):
        return [(round(x * SCALE), round(y * SCALE)) for x, y in points]

    def line(self, points, color=BLUE, width=2):
        self.d.line(self.points(points), fill=color, width=round(width * SCALE), joint="curve")

    def stroke(self, points, color=BLUE, width=2):
        self.line(points, INK, width + 2.5)
        self.line(points, WHITE, width + 1.2)
        self.line(points, color, width)

    def polygon(self, points, color=WHITE, outline=INK, width=1):
        p = self.points(points)
        self.d.polygon(p, fill=color)
        if outline:
            self.d.line(p + [p[0]], fill=outline, width=round(width * SCALE), joint="curve")

    def ellipse(self, box, color=WHITE, outline=INK, width=1):
        self.d.ellipse(tuple(round(v * SCALE) for v in box), fill=color, outline=outline, width=round(width * SCALE))

    def circle(self, x, y, r, color=WHITE, outline=INK, width=1):
        self.ellipse((x - r, y - r, x + r, y + r), color, outline, width)

    def rect(self, box, color=WHITE, outline=INK, radius=2, width=1):
        self.d.rounded_rectangle(tuple(round(v * SCALE) for v in box), radius=round(radius * SCALE), fill=color, outline=outline, width=round(width * SCALE))

    def arc(self, x, y, r, start, length, color=BLUE, width=2):
        pts = [(x + r * math.cos(start + length * i / 48), y + r * math.sin(start + length * i / 48)) for i in range(49)]
        self.line(pts, color, width)

    def finish(self):
        result = self.image.resize((CELL, CELL), Image.Resampling.LANCZOS)
        a = np.array(result)
        a[a[:, :, 3] == 0] = 0
        # One transparent guard pixel prevents a neighbouring atlas cell bleeding in.
        a[0, :] = a[-1, :] = a[:, 0] = a[:, -1] = 0
        return Image.fromarray(a)


def mascot(c, source, t, x=25, y=3, size=51, sway=0.0):
    body, _, _ = source
    sprite = body.resize((round(size * SCALE), round(size * SCALE)), Image.Resampling.LANCZOS)
    c.image.alpha_composite(sprite, (round((x + sway * math.sin(TAU * t)) * SCALE), round(y * SCALE)))


def arrow(c):
    c.polygon([(10, 12), (13, 37), (19, 31), (25, 42), (30, 39), (24, 28), (33, 27)], WHITE, INK, 1.6)
    c.polygon([(13, 17), (15, 32), (20, 27), (27, 27)], BLUE, None)


def star(c, x, y, r=3, color=CYAN):
    c.polygon([(x, y-r), (x+r*.32,y-r*.32),(x+r,y),(x+r*.32,y+r*.32),(x,y+r),(x-r*.32,y+r*.32),(x-r,y),(x-r*.32,y-r*.32)], color, INK, .6)


def badge(c, x, y, sign, color=BLUE):
    c.circle(x, y, 7.5, WHITE, INK, 1.2)
    c.circle(x, y, 5.5, color, None)
    c.line([(x-3,y),(x+3,y)], WHITE, 1.5)
    if sign == "+":
        c.line([(x,y-3),(x,y+3)], WHITE, 1.5)


def diamond(c, x, y):
    # Keep moving shuttles outside the downsampling filter's footprint at the anchor.
    c.circle(x,y,5.5,WHITE,None)
    c.polygon([(x,y-2.5),(x+2.5,y),(x,y+2.5),(x-2.5,y)], WHITE, INK, .8)


def arrowhead(c, x, y, angle, color=BLUE, size=6):
    u, v = (math.cos(angle), math.sin(angle)), (-math.sin(angle), math.cos(angle))
    c.polygon([(x+u[0]*size,y+u[1]*size), (x-u[0]*2+v[0]*4,y-u[1]*2+v[1]*4), (x-u[0]*2-v[0]*4,y-u[1]*2-v[1]*4)], color, INK, 1)


def document(c, x, y, color=WHITE):
    c.rect((x,y,x+19,y+24), color, INK, 2, 1.1)
    for off, length in [(7,11),(11,11),(15,7)]:
        c.line([(x+4,y+off),(x+4+length,y+off)], LILAC, 1.2)


def hand(c, x, y, closed=False, bend=0):
    if closed:
        c.rect((x-10,y-8,x+9,y+7), WHITE, INK, 4, 1.4)
        for off in [-6,-1,4]:
            c.line([(x+off,y-7),(x+off,y-2)], LILAC, 1)
        c.rect((x-9,y+4,x+5,y+12), BLUE, INK, 2, 1.2)
        c.line([(x-9,y+1),(x-3,y+3),(x+2,y-1)], INK, 1.2)
    else:
        c.polygon([(x-7,y+8),(x-13,y-1),(x-13,y-5),(x-10,y-6),(x-6,y-2),(x-7,y-15+bend),(x-4,y-17+bend),(x-1,y-14+bend),(x-1,y-19),(x+2,y-20),(x+4,y-17),(x+5,y-15+bend),(x+8,y-16+bend),(x+10,y-12+bend),(x+10,y),(x+6,y+9)], WHITE, INK, 1.3)
        c.rect((x-7,y+7,x+6,y+13), BLUE, INK, 2, 1)
        c.line([(x-5,y+2),(x,y+4),(x+5,y+1)], LILAC, 1)


def render(spec, frame, source):
    t = frame / spec.frames
    s, co = math.sin(TAU*t), math.cos(TAU*t)
    c, name = Canvas(), spec.name
    precision = name in ("Text", "VerticalText", "Cell", "Crosshair")
    mascot(c, source, t, x=30 if precision else 24, y=3, size=43 if precision else 51)

    if name in ("Default", "ContextMenu", "Help", "Progress"):
        if name == "Default":
            pts = [(20+10*u,39+24*u+1.6*math.sin(TAU*(t-u))) for u in np.linspace(0,1,32)]
            c.stroke(pts, LILAC, 2)
            star(c, 31+2*s, 65+co, 2.5)
        elif name == "ContextMenu":
            c.rect((30,44,58,69), WHITE, INK, 3, 1.2)
            scan=1-co
            for row in range(3):
                yy = 50+7*row
                weight=max(0,1-abs(scan-row))
                color=tuple(round(a+(b-a)*weight) for a,b in zip((218,212,248),(71,137,255)))
                c.rect((34,yy-2,54,yy+2), color, None, 1)
            c.circle(37+14*(.5-.5*co), 50+7*scan, 1.1, WHITE, None)
        elif name == "Help":
            x, y = 39+1.3*s, 53
            c.arc(x,y-3,6,math.pi,math.pi*1.5,BLUE,3)
            c.stroke([(x,y+3),(x,y+7)],BLUE,2.5)
            c.circle(39,67,2,BLUE,INK,.8)
            star(c,55,53+3*co,2)
        else:
            c.circle(39,56,12,WHITE,INK,1.2)
            for j in range(3):
                a=TAU*(t-j*.13)
                c.circle(39+8*math.cos(a),56+8*math.sin(a),2.5-j*.4,[BLUE,LILAC,CYAN][j],None)
            diamond(c,39,56)
        arrow(c)
    elif name == "Pointer":
        c.ellipse((7,54,42,68),"#DAD3F7",INK,1)
        c.ellipse((10,53,39,63),WHITE,INK,1)
        c.arc(24,58,6+8*t,0,TAU-0.02,BLUE,1.2)
        c.polygon([(15,58),(15,40),(17,34),(20,32),(23,34),(23,47),(26,43),(30,44),(32,48),(35,47),(38,50),(37,60),(31,64),(21,62)],WHITE,INK,1.5)
        c.line([(24,51),(26,55)],LILAC,1)
        # The finger and the contact anchor are redrawn after the travelling wave.
        c.circle(20,34,1.2,WHITE,None)
    elif name in ("Text","VerticalText"):
        if name == "Text":
            c.stroke([(17,29),(17,65)],WHITE,2)
            for y in (29,65): c.stroke([(11,y),(23,y)],BLUE,1.5)
            c.circle(17,31+32*t,1.2,CYAN,None)
            c.circle(17,31+32*((t+.5)%1),1.2,CYAN,None)
            c.stroke([(17,42),(17,52)],WHITE,2)
        else:
            c.stroke([(10,54),(46,54)],WHITE,2)
            for x in (10,46): c.stroke([(x,48),(x,60)],BLUE,1.5)
            c.circle(12+32*t,54,1.2,CYAN,None)
            c.circle(12+32*((t+.5)%1),54,1.2,CYAN,None)
            c.stroke([(23,54),(33,54)],WHITE,2)
    elif name == "Cell":
        for row in range(2):
            for col in range(2):
                x,y=12+14*col,40+14*row
                active=(row*2+col)==int(4*t)
                c.rect((x,y,x+12,y+12),"#B9DAFF" if active else WHITE,INK,1,1)
                if active: c.line([(x+2,y+9),(x+9,y+2)],CYAN,1.4)
        c.stroke([(26,46),(26,62)],BLUE,1.5)
        c.stroke([(18,54),(34,54)],BLUE,1.5)
        diamond(c,26,54)
    elif name == "Crosshair":
        for j in range(4): c.arc(27,53,15,TAU*(t+j/4),.42,BLUE,1.8)
        for points in [[(8,53),(22,53)],[(32,53),(46,53)],[(27,34),(27,48)],[(27,58),(27,72)]]: c.stroke(points,WHITE,1)
        c.circle(27,53,1.2,WHITE,INK,.6)
    elif name in ("Grab","Grabbing","PanReady","Panning"):
        x,y=spec.hotspot
        if name == "Grab":
            c.arc(x,y+9,16,.2+.1*s,1.1,LILAC,1.3)
            hand(c,x,y,False,.7*s)
        elif name == "Grabbing":
            c.rect((x-16,y-4-s,x+16,y+5+s),CYAN,INK,4,1)
            hand(c,x,y,True)
            for side in (-1,1): c.line([(x+side*15,y+8),(x+side*(13+1.5*co),y+11)],BLUE,1.5)
        elif name == "PanReady":
            c.arc(x,y,19,TAU*t,2.3,LILAC,1.6)
            hand(c,x,y-2,False,.4*s)
        else:
            c.rect((7,36,45,71),"#E2E7FF",INK,2,1)
            for row in range(3):
                yy=39+10*row
                c.line([(9,yy+2*s),(22,yy-2+2*s),(42,yy+3+2*s)],LILAC,1)
            hand(c,x,y,True)
        diamond(c,x,y)
    elif name in ("Move","AllScroll"):
        x,y=29,53
        for j in range(4):
            a=TAU*j/4
            c.stroke([(x,y),(x+13*math.cos(a),y+13*math.sin(a))],BLUE,2)
            arrowhead(c,x+13*math.cos(a),y+13*math.sin(a),a,size=5)
            if name == "Move":
                r=4+7*(.5-.5*co)
                c.circle(x+r*math.cos(a),y+r*math.sin(a),1.6,CYAN,None)
        if name == "AllScroll":
            a=TAU*t
            c.circle(x+12*math.cos(a),y+12*math.sin(a),2.2,CYAN,INK,.6)
        diamond(c,x,y)
    elif name == "Copy":
        shift=3+4*(.5-.5*co)
        document(c,15+shift,37-shift,"#C9D8FF")
        document(c,12,42)
        badge(c,24,54,"+")
    elif name == "Alias":
        points=[(12,66),(12,49),(24,44),(39,44),(44,49)]
        c.stroke(points,BLUE,3)
        arrowhead(c,44,49,math.pi/2,size=5)
        route=[(12,66),(12,49),(24,44),(39,44),(44,49)]
        seg=min(int(t*4),3); p=t*4-seg
        a,b=route[seg:seg+2]
        c.circle(a[0]+(b[0]-a[0])*p,a[1]+(b[1]-a[1])*p,2*math.sin(math.pi*t),CYAN,None)
        diamond(c,24,55)
    elif name in ("NoDrop","NotAllowed"):
        if name == "NoDrop":
            dy=3*(.5-.5*co)
            c.rect((11,33-dy,32,47-dy),WHITE,INK,2,1.1)
            c.line([(21,34-dy),(21,45-dy)],LILAC,1.5)
        else:
            a=TAU*t
            c.circle(25+17*math.cos(a),54+17*math.sin(a),2,LILAC,INK,.5)
        c.circle(25,54,12,WHITE,INK,1.3)
        c.circle(25,54,9,None,RED,2.4)
        c.line([(18,47),(32,61)],RED,3)
    elif "Resize" in name:
        x,y=28,54
        angles={"EResize":0,"NResize":-math.pi/2,"NeResize":-math.pi/4,"NwResize":-3*math.pi/4,"SResize":math.pi/2,"SeResize":math.pi/4,"SwResize":3*math.pi/4,"WResize":math.pi,"EwResize":0,"NsResize":math.pi/2,"NeswResize":-math.pi/4,"NwseResize":math.pi/4,"ColResize":0,"RowResize":math.pi/2}
        a=angles[name]; ux,uy=math.cos(a),math.sin(a); vx,vy=-uy,ux
        double=name in ("EwResize","NsResize","NeswResize","NwseResize","ColResize","RowResize")
        c.stroke([(x-ux*13,y-uy*13),(x+ux*13,y+uy*13)],BLUE,2)
        arrowhead(c,x+ux*13,y+uy*13,a,size=5)
        if double: arrowhead(c,x-ux*13,y-uy*13,a+math.pi,size=5)
        else: c.stroke([(x-ux*13-vx*4,y-uy*13-vy*4),(x-ux*13+vx*4,y-uy*13+vy*4)],LILAC,1)
        if name in ("ColResize","RowResize"):
            for side in (-1,1):
                r=3.5*side
                c.stroke([(x+ux*r-vx*13,y+uy*r-vy*13),(x+ux*r+vx*13,y+uy*r+vy*13)],WHITE,1)
        for side in ((-1,1) if double else (1,)):
            r=(4+6*(.5-.5*co))*side if double else -9+18*t
            radius=1.7 if double else 1.7*math.sin(math.pi*t)
            c.circle(x+ux*r,y+uy*r,radius,CYAN,None)
        diamond(c,x,y)
    elif name in ("ZoomIn","ZoomOut"):
        c.stroke([(34,61),(43,70)],LILAC,4)
        c.circle(25,52,13,WHITE,INK,1.4)
        c.circle(25,52,10,"#DCEFFF",BLUE,1.2)
        r=6+3*(t if name=="ZoomIn" else 1-t)
        weight=math.sin(math.pi*t)
        c.circle(25,52,r,None,tuple(round(a+(b-a)*weight) for a,b in zip((220,239,255),(100,160,240))),1)
        c.circle(25,52,5,WHITE,None)
        c.line([(20,52),(30,52)],INK,2)
        if name=="ZoomIn": c.line([(25,47),(25,57)],INK,2)
    elif name == "Wait":
        # The retained high-resolution rig is used directly, never a rescaled 64px atlas.
        c = Canvas()
        cx,cy=25,56
        c.circle(cx,cy,17,WHITE,INK,1.2)
        c.circle(cx,cy,12,None,BLUE,2)
        for j in range(5):
            c.arc(cx,cy,14,TAU*(t+j/5),.7,[BLUE,LILAC,CYAN,BLUE,LILAC][j],2)
        body,arm,empty=source
        size=61*SCALE
        c.image.alpha_composite(body.resize((size,size),Image.Resampling.LANCZOS),(14*SCALE,1*SCALE))
        beat=(.5-.5*math.cos(TAU*t))**2
        angle=math.radians(31-14*beat); sc=1.4
        aa,bb=math.cos(angle)/sc,math.sin(angle)/sc
        dd,ee=-bb,aa; px,py=450,803; dx,dy=470+10*beat,800
        held=arm.copy()
        if .34<t<.72: held.alpha_composite(empty)
        moved=held.transform(held.size,Image.Transform.AFFINE,(aa,bb,px-aa*dx-bb*dy,dd,ee,py-dd*dx-ee*dy),Image.Resampling.BICUBIC)
        c.image.alpha_composite(moved.resize((size,size),Image.Resampling.LANCZOS),(14*SCALE,1*SCALE))
        diamond(c,cx,cy)
    elif name in ("AddSelection","RemoveSelection"):
        c.rect((8,39,39,65),None,INK,2,1)
        c.rect((11,42,22,52),"#D2E4FF",BLUE,1,1)
        r=(.5-.5*co) if name=="AddSelection" else (.5+.5*co)
        c.rect((30-5*r,35+5*r,40-5*r,45+5*r),CYAN,INK,1,1)
        badge(c,26,54,"+" if name=="AddSelection" else "-",BLUE if name=="AddSelection" else RED)
    elif name == "RangeSelection":
        end=29+13*(.5-.5*co)
        c.rect((16,44,end,62),"#DEE5FF",BLUE,1,1)
        for i in range(6): c.line([(17+5*i,65),(17+5*i,68 if i%2 else 70)],INK,1)
        c.stroke([(16,56),(end,56)],BLUE,1.5)
        c.circle(end,56,2,CYAN,INK,.5)
        diamond(c,16,56)
    elif name == "DragSort":
        dy=8*s
        c.rect((9,40+dy,40,46+dy),"#D6CFF7",INK,1,1)
        c.rect((9,57-dy,40,63-dy),"#BDDCFF",INK,1,1)
        for yy in (40+dy,57-dy): c.line([(14,yy+3),(33,yy+3)],WHITE,1)
        c.rect((21,49,31,59),WHITE,INK,2,1)
        for xx in (24,28):
            for yy in (52,56): c.circle(xx,yy,.6,BLUE,None)
        diamond(c,26,54)
    elif name == "InsertDrop":
        dy=5*(.5-.5*co)
        c.rect((15,34+dy,35,45+dy),WHITE,INK,2,1)
        c.stroke([(25,45+dy),(25,50)],BLUE,1.6)
        arrowhead(c,25,48,math.pi/2,size=4)
        c.stroke([(9,56),(41,56)],BLUE,2)
        for x in (9,41): c.line([(x,51),(x,61)],INK,1.2)
        diamond(c,25,56)
    elif name == "ReplaceDrop":
        document(c,10,42,"#D7D1ED")
        shift=7*(.5-.5*co)
        document(c,25-shift,34+shift,"#D1E7FF")
        badge(c,25,54,"-")
        c.line([(22,52),(28,56)],WHITE,1.4)
        arrowhead(c,29,53,0,WHITE,3)
    elif name in ("HorizontalValueAdjust","VerticalValueAdjust"):
        vertical=name=="VerticalValueAdjust"
        def xy(a,b): return (28+b,54+a) if vertical else (28+a,54+b)
        c.stroke([xy(-17,0),xy(17,0)],BLUE,1.6)
        for j in range(-3,4):
            p=j*5+2.4*s
            c.line([xy(p,4),xy(p,8 if j%2 else 11)],LILAC,1)
        for side in (-1,1):
            x,y=xy(15*side,0); arrowhead(c,x,y,(math.pi/2 if vertical else 0)+(math.pi if side<0 else 0),size=4)
        c.rect((24,50,32,58),WHITE,INK,2,1)
        diamond(c,28,54)
    elif name == "ReadOnly":
        document(c,8,36,"#E7ECFF")
        c.line([(11,39+20*t),(24,39+20*t)],CYAN,1.7)
        c.arc(25,49,7,math.pi,math.pi,INK,3)
        c.arc(25,49,7,math.pi,math.pi,WHITE,1.5)
        c.rect((15,49,35,66),WHITE,INK,3,1.3)
        c.rect((18,52,32,63),"#C4DFFF",None,2)
        c.circle(25,55,2,INK,None)
        c.line([(25,56),(25,60)],INK,2)
        star(c,38,44+2*s,2.2)
    else:
        raise ValueError(f"Unimplemented semantic: {name}")
    return c.finish()


def snake(name):
    import re
    return re.sub(r"(?<!^)(?=[A-Z])", "_", name).lower()


def rust_catalogue(specs):
    variants="\n".join(f"    {s.name}," for s in specs)
    all_values=",\n".join(f"        Self::{s.name}" for s in specs)
    metadata="\n".join(f"            Self::{s.name} => ({s.frames}, {s.millis}, ({s.hotspot[0]}, {s.hotspot[1]}))," for s in specs)
    paths="\n".join(f'            Self::{s.name} => embedded_path!("assets/cursors/kamuri/{snake(s.name)}_80.png"),' for s in specs)
    data="\n".join(f'    asset_data!("assets/cursors/kamuri/{snake(s.name)}_80.png"),' for s in specs)
    return f'''// Generated by tools/build_kamuri_animations.py from the authored animation specifications.
use bevy::{{asset::{{AssetPath, embedded_path}}, prelude::*}};
use std::time::Duration;
use crate::constants::EMBEDDED_SOURCE;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BuiltinCursorAnimation {{
{variants}
}}

impl BuiltinCursorAnimation {{
    pub const ALL: [Self; {len(specs)}] = [
{all_values}
    ];

    fn metadata(self) -> (usize, u64, (u16, u16)) {{
        match self {{
{metadata}
        }}
    }}

    pub fn frame_count(self) -> usize {{ self.metadata().0 }}
    pub fn frame_duration(self) -> Duration {{ Duration::from_millis(self.metadata().1) }}
    pub fn hotspot(self) -> (u16, u16) {{ self.metadata().2 }}
    pub fn frame_at(self, elapsed: Duration) -> usize {{
        (elapsed.as_nanos() / self.frame_duration().as_nanos() % self.frame_count() as u128) as usize
    }}
    pub fn size(self) -> UVec2 {{
        UVec2::new(640, self.frame_count().div_ceil(8) as u32 * 80)
    }}
    pub fn atlas_layout(self) -> TextureAtlasLayout {{
        let mut layout = TextureAtlasLayout::from_grid(UVec2::splat(80), 8, self.frame_count().div_ceil(8) as u32, None, None);
        layout.textures.truncate(self.frame_count());
        layout
    }}
    pub fn path(self) -> AssetPath<'static> {{
        let path = match self {{
{paths}
        }};
        AssetPath::from_path_buf(path).with_source(EMBEDDED_SOURCE)
    }}
}}

macro_rules! asset_data {{
    ($path:literal) => {{ ($path, include_bytes!($path) as &[u8]) }};
}}
pub(crate) const KAMURI_ANIMATIONS: [(&str, &[u8]); {len(specs)}] = [
{data}
];
'''


def build(args):
    out=args.output
    out.mkdir(parents=True,exist_ok=True)
    source=tuple(Image.open(Path(__file__).parent/"wait"/n).convert("RGBA") for n in ("body.png","arm.png","empty_spoon.png"))
    specs=[s for s in SPECS if not args.samples or s.name in ("Default","Text","ZoomIn","Wait")]
    report=[]
    static=Image.new("RGBA",(640,480))
    contact=Image.new("RGB",(9*160,5*134),"#252A40")
    draw=ImageDraw.Draw(contact)
    for spec in specs:
        frames=[render(spec,i,source) for i in range(spec.frames)]
        atlas=Image.new("RGBA",(640,math.ceil(spec.frames/8)*80))
        for i,f in enumerate(frames): atlas.paste(f,(i%8*80,i//8*80))
        slug=snake(spec.name)
        atlas.save(out/f"{slug}_80.png",optimize=True)
        i=SPECS.index(spec)
        static.paste(frames[0],(i%8*80,i//8*80))
        frames[0].save(out/f"{slug}.apng",save_all=True,append_images=frames[1:],duration=spec.millis,loop=0,disposal=0,blend=0)
        motion=Image.new("RGBA",(800,100),"#252A40")
        for k in range(8): motion.alpha_composite(frames[k*len(frames)//8],(k*100+10,10))
        motion.convert("RGB").save(out/f"{slug}_motion.png")
        thumb=Image.new("RGBA",(160,100),"#252A40")
        thumb.alpha_composite(frames[0].resize((96,96)),(32,0))
        contact.paste(thumb.convert("RGB"),(i%9*160,i//9*134))
        draw.text((i%9*160+8,i//9*134+101),spec.name,fill="#ECE6FF")
        draw.text((i%9*160+8,i//9*134+116),f"{spec.frames}f / {spec.millis}ms",fill="#A7BCEB")
        anchor=(spec.hotspot[0]-1,spec.hotspot[1]-1,spec.hotspot[0]+2,spec.hotspot[1]+2)
        anchors={f.crop(anchor).tobytes() for f in frames}
        report.append({"name":spec.name,"atlas":f"{slug}_80.png","preview":f"{slug}.apng","frame_size":[80,80],"columns":8,"frames":spec.frames,"frame_duration_ms":spec.millis,"hotspot":spec.hotspot,"anchor_rect":anchor,"motion":spec.motion,"unique_frames":len({f.tobytes() for f in frames}),"anchor_variants":len(anchors),"sha256":hashlib.sha256((out/f"{slug}_80.png").read_bytes()).hexdigest()})
    static.save(out/"static_atlas_80.png",optimize=True)
    contact.save(out/"contact-sheet.png")
    (out/"manifest.json").write_text(json.dumps({"theme":"Kamuri","loop":True,"animations":report},indent=2),encoding="utf-8")
    cards="\n".join(f'<article><img src="{r["preview"]}" width="80" height="80"><div><b>{html.escape(r["name"])}</b><small>{r["frames"]} frames · {r["frame_duration_ms"]}ms · hotspot {r["hotspot"]}</small><p>{html.escape(r["motion"])}</p></div></article>' for r in report)
    (out/"index.html").write_text('''<!doctype html><meta charset="utf-8"><title>Kamuri · Motion study</title><style>body{margin:32px;background:#181b2a;color:#f4efff;font:15px system-ui}h1{font-weight:500}main{display:grid;grid-template-columns:repeat(auto-fit,minmax(330px,1fr));gap:12px}article{display:flex;gap:14px;background:#272b40;padding:16px;border-radius:12px;align-items:center}img{flex:0 0 80px;background:repeating-conic-gradient(#33394d 0 25%,#2c3144 0 50%) 0/16px 16px}small{display:block;color:#b0c8fa}p{font-size:12px;color:#d1c9eb;margin:6px 0}button{margin:12px;padding:8px}</style><h1>Kamuri · Motion study</h1><p>45 semantic loops · 80 × 80 · fixed anchors · same atlases as the native cursor</p><button onclick="document.querySelectorAll('img').forEach(i=>i.style.background='#e8eaf4')">Light</button><button onclick="document.querySelectorAll('img').forEach(i=>i.style.background='#202336')">Dark</button><main>'''+cards+"</main>",encoding="utf-8")
    if args.install:
        if args.samples: raise ValueError("Sample runs cannot install an incomplete catalogue")
        if any(r["anchor_variants"]!=1 for r in report):
            raise ValueError("Functional anchors must be stable before installation")
        dest=ROOT/"crates/asset/src/assets/cursors/kamuri"
        dest.mkdir(parents=True,exist_ok=True)
        for name in [r["atlas"] for r in report]+["static_atlas_80.png"]: (dest/name).write_bytes((out/name).read_bytes())
        (ROOT/"crates/asset/src/cursor_animation.rs").write_text(rust_catalogue(SPECS),encoding="utf-8")
    print(json.dumps({"output":str(out),"count":len(report),"anchor_issues":[r["name"] for r in report if r["anchor_variants"]!=1]}))


if __name__=="__main__":
    parser=argparse.ArgumentParser()
    parser.add_argument("--output",type=Path,default=ROOT/"target/animation/kamuri80")
    parser.add_argument("--samples",action="store_true")
    parser.add_argument("--install",action="store_true")
    build(parser.parse_args())
