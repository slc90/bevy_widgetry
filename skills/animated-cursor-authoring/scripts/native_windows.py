"""Capture GetCursorInfo/GetIconInfo evidence from the Windows desktop.

The sample mode opens a disposable native window. The capture mode observes an
existing app without changing ECS state. Use an interactive Windows desktop. Input positions are physical client pixels.
"""

import argparse
import bisect
import re
import sys
import ctypes as C
from ctypes import wintypes as W
import hashlib
import io
import json
from pathlib import Path
import struct
import time

from PIL import Image
import numpy as np


if sys.platform != "win32":
    raise SystemExit("native_windows.py requires an interactive Windows desktop")


user = C.WinDLL("user32", use_last_error=True)
gdi = C.WinDLL("gdi32", use_last_error=True)
kernel = C.WinDLL("kernel32", use_last_error=True)
user.SetProcessDpiAwarenessContext(C.c_void_p(-4))
HANDLE = C.c_void_p
LRESULT = C.c_ssize_t
WNDPROC = C.WINFUNCTYPE(LRESULT, W.HWND, W.UINT, W.WPARAM, W.LPARAM)


class CURSORINFO(C.Structure):
    _fields_ = [("cbSize", W.DWORD), ("flags", W.DWORD), ("hCursor", HANDLE), ("pos", W.POINT)]


class ICONINFO(C.Structure):
    _fields_ = [("fIcon", W.BOOL), ("xHotspot", W.DWORD), ("yHotspot", W.DWORD), ("hbmMask", HANDLE), ("hbmColor", HANDLE)]


class BITMAP(C.Structure):
    _fields_ = [("type", W.LONG), ("width", W.LONG), ("height", W.LONG), ("stride", W.LONG), ("planes", W.WORD), ("bits", W.WORD), ("data", HANDLE)]


class HEADER(C.Structure):
    _fields_ = [("size", W.DWORD), ("width", W.LONG), ("height", W.LONG), ("planes", W.WORD), ("bits", W.WORD), ("compression", W.DWORD), ("sizeImage", W.DWORD), ("x", W.LONG), ("y", W.LONG), ("used", W.DWORD), ("important", W.DWORD)]


class WNDCLASS(C.Structure):
    _fields_ = [("style", W.UINT), ("proc", WNDPROC), ("clsExtra", C.c_int), ("wndExtra", C.c_int), ("instance", HANDLE), ("icon", HANDLE), ("cursor", HANDLE), ("brush", HANDLE), ("menu", W.LPCWSTR), ("name", W.LPCWSTR)]


def api(dll, name, result, *args):
    fn = getattr(dll, name)
    fn.restype, fn.argtypes = result, args
    return fn


get_cursor = api(user,"GetCursorInfo",W.BOOL,C.POINTER(CURSORINFO))
get_icon = api(user,"GetIconInfo",W.BOOL,HANDLE,C.POINTER(ICONINFO))
get_object = api(gdi,"GetObjectW",C.c_int,HANDLE,C.c_int,HANDLE)
delete_object = api(gdi,"DeleteObject",W.BOOL,HANDLE)
get_dc = api(user,"GetDC",HANDLE,W.HWND)
release_dc = api(user,"ReleaseDC",C.c_int,W.HWND,HANDLE)
get_dib = api(gdi,"GetDIBits",C.c_int,HANDLE,HANDLE,W.UINT,W.UINT,HANDLE,HANDLE,W.UINT)
set_cursor = api(user,"SetCursor",HANDLE,HANDLE)
load_cursor = api(user,"LoadImageW",HANDLE,HANDLE,W.LPCWSTR,W.UINT,C.c_int,C.c_int,W.UINT)
destroy_cursor = api(user,"DestroyCursor",W.BOOL,HANDLE)
set_pos = api(user,"SetCursorPos",W.BOOL,C.c_int,C.c_int)
get_pos = api(user,"GetCursorPos",W.BOOL,C.POINTER(W.POINT))
find_window = api(user,"FindWindowW",W.HWND,W.LPCWSTR,W.LPCWSTR)
client_to_screen = api(user,"ClientToScreen",W.BOOL,W.HWND,C.POINTER(W.POINT))
foreground = api(user,"SetForegroundWindow",W.BOOL,W.HWND)
api(user,"DefWindowProcW",LRESULT,W.HWND,W.UINT,W.WPARAM,W.LPARAM)
api(kernel,"GetModuleHandleW",HANDLE,W.LPCWSTR)
api(user,"RegisterClassW",W.WORD,C.POINTER(WNDCLASS))
api(user,"CreateWindowExW",W.HWND,W.DWORD,W.LPCWSTR,W.LPCWSTR,W.DWORD,C.c_int,C.c_int,C.c_int,C.c_int,W.HWND,HANDLE,HANDLE,HANDLE)
api(user,"DestroyWindow",W.BOOL,W.HWND)
api(user,"ShowWindow",W.BOOL,W.HWND,C.c_int)
api(user,"SetWindowPos",W.BOOL,W.HWND,W.HWND,C.c_int,C.c_int,C.c_int,C.c_int,W.UINT)
api(user,"GetWindowLongW",W.LONG,W.HWND,C.c_int)
api(user,"PeekMessageW",W.BOOL,C.POINTER(W.MSG),W.HWND,W.UINT,W.UINT,W.UINT)
api(user,"DispatchMessageW",LRESULT,C.POINTER(W.MSG))


def check(ok, operation):
    if not ok:
        raise OSError(C.get_last_error(), operation)
    return ok


def snapshot():
    ci = CURSORINFO(cbSize=C.sizeof(CURSORINFO))
    check(get_cursor(C.byref(ci)),"GetCursorInfo")
    ii = ICONINFO()
    check(get_icon(ci.hCursor,C.byref(ii)),"GetIconInfo")
    try:
        bm=BITMAP()
        check(get_object(ii.hbmColor or ii.hbmMask,C.sizeof(bm),C.byref(bm)),"GetObject")
        height=bm.height if ii.hbmColor else bm.height//2
        info={"hotspot":[ii.xHotspot,ii.yHotspot],"size":[bm.width,height],"position":[ci.pos.x,ci.pos.y],"visible":bool(ci.flags&1),"handle":ci.hCursor}
        if not ii.hbmColor:
            return info,None
        header=HEADER(size=C.sizeof(HEADER),width=bm.width,height=-height,planes=1,bits=32)
        pixels=C.create_string_buffer(bm.width*height*4)
        dc=check(get_dc(None),"GetDC")
        try:
            check(get_dib(dc,ii.hbmColor,0,height,pixels,C.byref(header),0),"GetDIBits")
        finally:
            release_dc(None,dc)
        image=Image.frombytes("RGBA",(bm.width,height),pixels.raw,"raw","BGRA")
        info["sha256"]=hashlib.sha256(pixels.raw).hexdigest()
        info["bitmap_format"]="BGRA8 from GetDIBits"
        return info,image
    finally:
        if ii.hbmColor: delete_object(ii.hbmColor)
        if ii.hbmMask: delete_object(ii.hbmMask)


def durations(item):
    result=item.get("durations_ms",[item.get("frame_duration_ms",0)]*item["frames"])
    if len(result)!=item["frames"] or any(not isinstance(v,int) or v<=0 for v in result):
        raise ValueError("Every frame needs a positive duration in milliseconds")
    return result


def folder(index, name):
    slug=re.sub(r"[^A-Za-z0-9_-]+","-",name).strip("-") or "cursor"
    return f"{index:03d}-{slug}"


def collect(seconds, pump=lambda:None, premultiplied=False):
    records,frames=[],[]
    start=time.perf_counter()
    while time.perf_counter()-start<seconds:
        pump()
        info,frame=snapshot()
        info["elapsed"]=time.perf_counter()-start
        records.append(info)
        if frame is not None:
            if premultiplied:
                frame=Image.frombytes("RGBa",frame.size,frame.tobytes()).convert("RGBA")
            frames.append((frame,info["elapsed"]))
        time.sleep(.025)
    return records,frames


def save(out, records, frames):
    out.mkdir(parents=True,exist_ok=True)
    (out/"native.json").write_text(json.dumps(records,indent=2),encoding="utf-8")
    if frames:
        size=(max(f.width for f,_ in frames)+20,max(f.height for f,_ in frames)+20)
        previews=[]
        timing=[]
        for i,(frame,elapsed) in enumerate(frames):
            canvas=Image.new("RGBA",size,"#24283D")
            canvas.alpha_composite(frame,(10,10))
            previews.append(canvas)
            timing.append(max(1,round((frames[i+1][1]-elapsed)*1000)) if i+1<len(frames) else 25)
        previews[0].save(out/"native.apng",save_all=True,append_images=previews[1:],duration=timing,loop=0,disposal=0,blend=0)
        count=min(len(previews),12)
        strip=Image.new("RGBA",(count*size[0],size[1]))
        for i in range(count):
            strip.paste(previews[i*len(previews)//count],(i*size[0],0))
        strip.convert("RGB").save(out/"native-contact.png")
    print(json.dumps({"output":str(out),"samples":len(records),"unique":len({r.get('sha256',r['handle']) for r in records}),"sizes":sorted({tuple(r['size']) for r in records}),"hotspots":sorted({tuple(r['hotspot']) for r in records}),"visible":all(r['visible'] for r in records)}))


def capture_targets(args):
    hwnd=check(find_window(None,args.title),"FindWindow")
    if args.foreground:
        check(user.SetWindowPos(hwnd,W.HWND(-1),0,0,0,0,0x43),"SetWindowPos")
    foreground(hwnd)
    for ordinal,target in enumerate(json.loads(args.targets.read_text(encoding="utf-8"))):
        source=Image.open(args.targets.resolve().parent/target["atlas"]).convert("RGBA")
        width,height=target["frame_size"]
        columns=target["columns"]
        expected=set()
        for index in range(target["frames"]):
            x,y=index%columns*width,index//columns*height
            rgba=np.array(source.crop((x,y,x+width,y+height)),dtype=np.uint16)
            expected.add(hashlib.sha256(rgba[:,:,[2,1,0,3]].astype(np.uint8).tobytes()).hexdigest())
            for rounding in (0,127):
                converted=rgba.copy()
                converted[:,:,:3]=(rgba[:,:,:3]*rgba[:,:,3:4]+rounding)//255
                expected.add(hashlib.sha256(converted[:,:,[2,1,0,3]].astype(np.uint8).tobytes()).hexdigest())
        pt=W.POINT(*target["position"])
        check(client_to_screen(hwnd,C.byref(pt)),"ClientToScreen")
        check(set_pos(pt.x+2,pt.y+2),"SetCursorPos")
        time.sleep(.04)
        check(set_pos(pt.x,pt.y),"SetCursorPos")
        time.sleep(.5)
        seconds=sum(durations(target))/1000*2+.2
        records,frames=collect(seconds)
        for record in records:
            record["matches_source_frame"]=record.get("sha256") in expected
            record["semantic"]=target["name"]
        save(args.output/folder(ordinal,target["name"]),records,frames)
        if not all(r["visible"] and r["size"]==target["frame_size"] and r["hotspot"]==target["hotspot"] for r in records):
            raise ValueError(f'{target["name"]}: native cursor differs from the expected semantic')
        if len({r.get("sha256") for r in records})<2:
            raise ValueError(f'{target["name"]}: native cursor did not animate')
        if not all(r["matches_source_frame"] for r in records):
            raise ValueError(f'{target["name"]}: native pixels do not match the semantic atlas')


def sample(args):
    manifest=json.loads((args.samples/"manifest.json").read_text(encoding="utf-8"))
    current=[None]
    @WNDPROC
    def proc(hwnd,msg,wp,lp):
        if msg==0x20 and current[0]:
            set_cursor(current[0]); return 1
        return user.DefWindowProcW(hwnd,msg,wp,lp)
    instance=kernel.GetModuleHandleW(None)
    wc=WNDCLASS(proc=proc,instance=instance,brush=HANDLE(6),name="AnimatedCursorSampleProbe")
    check(user.RegisterClassW(C.byref(wc)),"RegisterClass")
    hwnd=check(user.CreateWindowExW(0,wc.name,"Animated cursor native sample probe",0x10CF0000,100,100,640,360,None,None,instance,None),"CreateWindow")
    original=W.POINT(); get_pos(C.byref(original))
    try:
        user.ShowWindow(hwnd,5)
        check(user.SetWindowPos(hwnd,W.HWND(-1),0,0,0,0,0x43),"SetWindowPos")
        foreground(hwnd)
        pos=W.POINT(260,150); check(client_to_screen(hwnd,C.byref(pos)),"ClientToScreen"); check(set_pos(pos.x,pos.y),"SetCursorPos")
        for ordinal,item in enumerate(manifest["animations"]):
            width,height=item["frame_size"]
            columns=item["columns"]
            timing=durations(item)
            cumulative=list(np.cumsum(timing))
            if not (1<=width<=256 and 1<=height<=256): raise ValueError("CUR sample output supports dimensions up to 256px")
            handles=[]
            atlas=Image.open(args.samples/item["atlas"]).convert("RGBA")
            out=args.output/folder(ordinal,item["name"])
            out.mkdir(parents=True,exist_ok=True)
            try:
                for i in range(item["frames"]):
                    x,y=i%columns*width,i//columns*height
                    f=atlas.crop((x,y,x+width,y+height)); data=io.BytesIO(); f.save(data,format="PNG"); png=data.getvalue()
                    hx,hy=item["hotspot"]
                    path=out/f"{i}.cur"
                    path.write_bytes(struct.pack("<HHHBBBBHHII",0,2,1,width%256,height%256,0,0,hx,hy,len(png),22)+png)
                    handles.append(check(load_cursor(None,str(path.resolve()),2,0,0,0x10),"LoadImage"))
                start=time.perf_counter()
                def pump():
                    current[0]=handles[bisect.bisect_right(cumulative,((time.perf_counter()-start)*1000)%sum(timing))]
                    msg=W.MSG()
                    while user.PeekMessageW(C.byref(msg),None,0,0,1):
                        user.TranslateMessage(C.byref(msg)); user.DispatchMessageW(C.byref(msg))
                    check(set_pos(pos.x,pos.y),"SetCursorPos")
                    set_cursor(current[0])
                settle=time.perf_counter()+.5
                while time.perf_counter()<settle:
                    pump(); time.sleep(.025)
                records,frames=collect(sum(timing)/1000*2+.15,pump,premultiplied=True)
                save(out,records,frames)
                if not all(r["visible"] and r["hotspot"]==item["hotspot"] and r["size"]==item["frame_size"] for r in records):
                    raise ValueError(f'{item["name"]}: unexpected native size, hotspot, or visibility')
                if len({r.get("sha256") for r in records})<2:
                    raise ValueError(f'{item["name"]}: native cursor did not animate')
            finally:
                current[0]=None; set_cursor(None)
                for h in handles: destroy_cursor(h)
    finally:
        user.DestroyWindow(hwnd); set_pos(original.x,original.y)


if __name__=="__main__":
    parser=argparse.ArgumentParser()
    parser.add_argument("--samples",type=Path)
    parser.add_argument("--targets",type=Path)
    parser.add_argument("--foreground",action="store_true")
    parser.add_argument("--output",type=Path,required=True)
    parser.add_argument("--seconds",type=float,default=6)
    parser.add_argument("--title")
    parser.add_argument("--position",type=int,nargs=2)
    args=parser.parse_args()
    if args.seconds<=0: parser.error("--seconds must be positive")
    if args.samples and args.targets: parser.error("Choose sample playback or existing-app capture")
    if (args.targets or args.position) and not args.title: parser.error("Existing-app capture requires --title")
    if args.samples:
        sample(args)
    elif args.targets:
        hwnd=check(find_window(None,args.title),"FindWindow")
        was_topmost=bool(user.GetWindowLongW(hwnd,-20)&8)
        try:
            capture_targets(args)
        finally:
            if args.foreground:
                hwnd=find_window(None,args.title)
                if hwnd: user.SetWindowPos(hwnd,W.HWND(-1 if was_topmost else -2),0,0,0,0,0x43)
    else:
        if args.position:
            hwnd=check(find_window(None,args.title),"FindWindow")
            foreground(hwnd)
            pt=W.POINT(*args.position); check(client_to_screen(hwnd,C.byref(pt)),"ClientToScreen"); check(set_pos(pt.x,pt.y),"SetCursorPos")
            time.sleep(.3)
        save(args.output,*collect(args.seconds))
