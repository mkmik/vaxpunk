+++
title = "Workstation graphics"
summary = "Display, keyboard and mouse drivers for a local DECwindows screen"
aka = ["framebuffer", "keyboard", "mouse", "pointer", "workstation", "VAXstation", "AlphaStation", "graphics board"]
kind = "core"
intent = "aspiration"
research = "deferred"
status = "dropped"

[[needs]]
fuzzy = "A framebuffer, a keyboard and a pointer device the PAL can reach"
why = "QEMU's virt machine would need ramfb or virtio-gpu and virtio-input"
+++

A VMS workstation drew DECwindows on its own screen: drivers for the
graphics board (QVSS and GPX on the VAXstation, TGA and PowerStorm
cards on Alpha), the keyboard and the mouse, under the local X server.
Without them a VMS system shows its X clients on another machine's X
server.

Marko has said there are no display drivers in vaxpunk: its X clients
talk to a remote X server, Xvnc on the Mac, and the work is Xlib, Xt,
Motif and UIL (t8y:/gui/x-server is dropped for the same reason).
