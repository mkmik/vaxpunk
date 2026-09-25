# seL4 kernel settings, loaded with `cmake -C`. The QEMU CPU, RAM and GIC
# version come from qemu.env through kernel/Makefile.
set(KernelPlatform qemu-arm-virt CACHE STRING "")
set(KernelSel4Arch aarch64 CACHE STRING "")
set(KernelArmHypervisorSupport OFF CACHE BOOL "") # Limine hands off at EL1
set(KernelMaxNumNodes 1 CACHE STRING "")
set(KernelVerificationBuild OFF CACHE BOOL "")
set(KernelDebugBuild ON CACHE BOOL "")
set(KernelPrinting ON CACHE BOOL "") # seL4_DebugPutChar
