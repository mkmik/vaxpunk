+++
title = "Web server"
summary = "An HTTP server on VMS: OSU, WASD, Apache (Secure Web Server)"
aka = ["WASD", "OSU DECthreads HTTP server", "CSWS", "Secure Web Server", "Apache"]
kind = "layered"
intent = "aspiration"
research = "deferred"
status = "todo"

[[needs]]
on = "/net/tcpip-qio"
why = "Listens on port 80"

[[needs]]
on = "/net/ssl"
why = "https needs the server side of TLS"
soft = true

[[needs]]
on = "/rms/core"
why = "Serves VMS files"
+++

VMS had web servers from the early 1990s: the OSU DECthreads HTTP
server and WASD were written for VMS. HP shipped Apache as
the Secure Web Server (CSWS), and VSI ships Apache too. None comes with
TCP/IP Services; they are separate products.

vaxpunk has none.
