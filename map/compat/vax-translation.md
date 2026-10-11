+++
title = "VAX binary translation"
summary = "Translate VAX/VMS images to native code, as VEST did for Alpha"
aka = ["VEST", "DECmigrate", "VAX Environment Software Translator", "TIE"]
kind = "layered"
intent = "aspiration"
research = "deferred"
status = "todo"

[[needs]]
on = "//exec/image-activator"
why = "Recognizes a translated image and sets up its environment"

[[needs]]
on = "//exec/calling-standard"
why = "Jackets between VAX CALLS frames and native calls"

[[needs]]
on = "//compat/alpha-translation"
why = "Shares the run-time environment, as AEST took VEST's images"
soft = true

[[needs]]
fuzzy = "A VAX instruction decoder with condition codes, VAX F, D, G and H floating, and a TIE-like run-time"
why = "The translator and its run-time"
+++

VEST, the VAX Environment Software Translator (sold as DECmigrate),
moved VAX/VMS customers to Alpha without recompiling: it turned a VAX
image into an Alpha image, keeping VAX semantics such as condition codes
and VAX floating point, and the Translated Image Environment handled at
run time what it couldn't translate ahead. AEST later carried such
images on to Itanium.

vaxpunk models Alpha VMS, not VAX, and Marko cares more about VMS's
feel than old binaries. vmacro already compiles VAX MACRO-32 sources to
ARM64; binary translation would be for VAX images without sources. Not
chosen.
