/* descrip.h: string descriptors. */
#ifndef __DESCRIP_LOADED
#define __DESCRIP_LOADED

#define DSC$K_DTYPE_T 14
#define DSC$K_CLASS_S 1

/* vaxpunk: C's pointers are 64-bit here, DEC C's 32-bit by default, so
 * an address in a VMS structure is a 32-bit integer: every address a
 * process has is below 2 GB. */
struct dsc$descriptor_s {
	unsigned short dsc$w_length;
	unsigned char dsc$b_dtype;
	unsigned char dsc$b_class;
	unsigned int dsc$a_pointer;
};

/* $DESCRIPTOR(name, "text"): a descriptor of a string constant.
 * vaxpunk: an automatic variable only, since a static initializer can't
 * cut an address to 32 bits. */
#define $DESCRIPTOR(name, string) \
	struct dsc$descriptor_s name = { sizeof(string) - 1, DSC$K_DTYPE_T, DSC$K_CLASS_S, \
					 (unsigned int)(unsigned long)(string) }

#endif
