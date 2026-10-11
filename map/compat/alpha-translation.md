+++
title = "Alpha binary translation"
summary = "Translate OpenVMS Alpha images to ARM64, as AEST did for Itanium"
aka = ["AEST", "TIE", "Translated Image Environment", "Alpha Environment Software Translator"]
kind = "layered"
intent = "aspiration"
research = "deferred"
status = "todo"
refs = ["docs/design/0004-calling-standard.md", "https://de.openvms.org/TUD2005/05_Binary_Translation_Andy_Goldstein.pdf"]

[[needs]]
on = "//exec/image-activator"
why = "Recognizes a translated image and sets up its environment"

[[needs]]
on = "//exec/calling-standard"
why = "Jackets between Alpha's calls and native ones; Alpha-style procedure descriptors help"

[[needs]]
on = "//exec/shareable-images"
why = "Translated images call native LIBRTL and system services"

[[needs]]
on = "//exec/condition-handling"
why = "Alpha's exceptions become VMS conditions in the translated code"

[[needs]]
fuzzy = "An Alpha instruction decoder and ARM64 code generator, a run-time environment like TIE that interprets what couldn't be translated ahead, and VAX F and G floating"
why = "The translator and its run-time"
+++

When OpenVMS moved to Itanium, HP gave customers without sources AEST,
the Alpha Environment Software Translator: it translated an Alpha image
into an Itanium one ahead of time, and the Translated Image Environment
(TIE) at run time interpreted any code it couldn't find statically,
jacketed calls between translated and native code, and emulated Alpha's
floating point and exceptions. It also took VAX images that VEST had
already turned into Alpha ones.

Marko would like vaxpunk to run Alpha VMS binaries as natively as
possible, by translation in this style, not by emulation. vaxpunk keeps
Alpha's image format, symbol vectors and procedure descriptor flags
(ADR-0028, DESIGN-0004), which would help. PRD-0003 lists running
Alpha binaries as a non-goal of Milestone 1.
