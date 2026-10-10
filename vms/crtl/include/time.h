/* time.h: the time of day, from the system time. */
#ifndef __TIME_LOADED
#define __TIME_LOADED

#include <decc$types.h>

typedef long time_t;

struct tm {
	int tm_sec, tm_min, tm_hour, tm_mday, tm_mon, tm_year, tm_wday, tm_yday, tm_isdst;
};

struct timeval {
	time_t tv_sec;
	long tv_usec;
};

int gettimeofday(struct timeval *tv, void *tz) __DECC(gettimeofday);
struct tm *gmtime(const time_t *t) __DECC(gmtime);
struct tm *gmtime_r(const time_t *t, struct tm *tm) __DECC(gmtime_r);
time_t time(time_t *t) __DECC(time);

#endif
