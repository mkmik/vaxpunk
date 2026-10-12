+++
title = "OpenVMS Galaxy"
summary = "Several VMS instances in partitions of one machine, sharing memory and CPUs"
aka = ["Galaxy", "OpenVMS Galaxy", "hard partitions", "soft partitions", "shared memory", "CPU migration", "Galaxy Configuration Utility", "GCU", "GALAXY parameter", "shared memory global sections"]
kind = "core"
intent = "aspiration"
research = "deferred"

[[feature]]
id = "instances"
title = "Several instances, each with its own CPUs, memory and devices, on one machine"
status = "todo"

[[feature]]
id = "shared-memory"
title = "Memory shared between instances: global sections and the shared memory cluster interconnect"
status = "todo"
needs = ["#instances", "/exec/global-sections"]

[[feature]]
id = "cpu-migration"
title = "CPUs moved from one instance to another while both run"
status = "todo"
needs = ["#instances", "/platform/smp"]

[[feature]]
id = "management"
title = "The Galaxy Configuration Utility and the GALAXY system parameter"
status = "todo"
needs = ["#instances"]

[[needs]]
on = "/exec/cluster"
why = "Instances usually form a cluster, over shared memory"
soft = true

[[needs]]
fuzzy = "A machine, or hypervisor, that can split its CPUs and memory between instances"
why = "Galaxy needed AlphaServer console support for partitions"
+++

OpenVMS Galaxy, from Alpha V7.2 in 1999, runs several instances of VMS
on one large AlphaServer, each in a partition with its own CPUs,
memory and I/O, the console firmware dividing the machine. Instances
share part of memory, through which they can share global sections
and form a cluster without a wire, and a CPU can be moved from one
instance to another while both run, so capacity follows the load.
Galaxy stayed on Alpha; Itanium VMS used the hardware's own
partitions instead.

vaxpunk runs one instance on one CPU. Under a hypervisor, several
vaxpunk VMs, as a cluster, would do much of what Galaxy did.
