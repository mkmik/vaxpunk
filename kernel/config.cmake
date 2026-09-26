# seL4 kernel settings, loaded with `cmake -C`. The QEMU CPU, RAM and GIC
# version come from qemu.env through kernel/Makefile.
set(KernelPlatform qemu-arm-virt CACHE STRING "")
set(KernelSel4Arch aarch64 CACHE STRING "")
set(KernelArmHypervisorSupport OFF CACHE BOOL "") # Limine hands off at EL1
set(KernelMaxNumNodes 1 CACHE STRING "")
set(KernelVerificationBuild OFF CACHE BOOL "")
set(KernelDebugBuild ON CACHE BOOL "")
set(KernelPrinting ON CACHE BOOL "") # seL4_DebugPutChar
# Mixed-criticality scheduling: threads run on scheduling context capabilities
# (budget and period) instead of a fixed timeslice, and replies go through
# reply objects. The root task gets seL4_CapInitThreadSC and the sched control
# caps in bootinfo->schedcontrol.
set(KernelIsMCS ON CACHE BOOL "")
