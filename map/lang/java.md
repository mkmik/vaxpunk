+++
title = "Java"
summary = "Java on OpenVMS: HP's JDK on Alpha and Itanium, VSI's OpenJDK on x86-64"
aka = ["Java", "JDK", "OpenJDK", "JVM", "HotSpot", "J2VMS"]
kind = "layered"
intent = "aspiration"
research = "deferred"
status = "todo"
refs = ["https://www.theregister.com/2023/04/13/openvms_921_x86_hobby/"]

[[needs]]
on = "//lang/cxx"
why = "The JVM is C++"

[[needs]]
on = "//rtl/crtl"
why = "The JVM runs on the C run-time library's POSIX side"

[[needs]]
on = "//rtl/pthreads"
why = "Java threads are POSIX threads"

[[needs]]
on = "//files/ods5"
why = "Class and package names need long, mixed-case file names"

[[needs]]
fuzzy = "A JVM port: a JIT or interpreter for ARM64, a garbage collector, and far more memory than 4 MB"
why = "The virtual machine itself"
+++

Sun's Java came to OpenVMS through DEC and Compaq, and HP kept a JDK on
Alpha and Itanium. VSI ships OpenJDK 8 for x86-64 since V9.2-1. Java
runs on the C run-time library's UNIX side, with POSIX threads, ODS-5
names and the `DECC$` feature logicals; J2VMS gives Java access to
system services. It is a layered product.

vaxpunk has none.
