/*
 * The C run-time library's time: the system time, in 100 ns units since
 * 17-NOV-1858 00:00, as seconds since 1-JAN-1970 00:00.
 *
 * ponytail: the system time is taken as UTC; DEC C subtracts
 * SYS$TIMEZONE_DIFFERENTIAL, which vaxpunk doesn't define yet.
 */
#include <starlet.h>
#include <time.h>

/* 1-JAN-1970 in the system's units. */
#define UNIX_EPOCH 35067168000000000LL

int gettimeofday(struct timeval *tv, void *tz)
{
	long long t;

	(void)tz;
	sys$gettim(&t);
	t -= UNIX_EPOCH;
	tv->tv_sec = t / 10000000;
	tv->tv_usec = t % 10000000 / 10;
	return 0;
}

time_t time(time_t *t)
{
	struct timeval tv;

	gettimeofday(&tv, NULL);
	if (t)
		*t = tv.tv_sec;
	return tv.tv_sec;
}

/* Howard Hinnant's civil_from_days, for the date of a day since 1970. */
struct tm *gmtime_r(const time_t *t, struct tm *tm)
{
	long days = *t / 86400, secs = *t % 86400;

	if (secs < 0)
		secs += 86400, days--;
	tm->tm_hour = secs / 3600;
	tm->tm_min = secs / 60 % 60;
	tm->tm_sec = secs % 60;
	tm->tm_wday = (int)((days % 7 + 11) % 7);	/* 1-JAN-1970 was a Thursday */
	long z = days + 719468, era = (z >= 0 ? z : z - 146096) / 146097;
	long doe = z - era * 146097;
	long yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
	long doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
	long mp = (5 * doy + 2) / 153;
	long d = doy - (153 * mp + 2) / 5 + 1;
	long m = mp < 10 ? mp + 3 : mp - 9;
	long y = yoe + era * 400 + (m <= 2);
	int leap = (y % 4 == 0 && y % 100 != 0) || y % 400 == 0;
	static const short before[] = { 0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334 };
	tm->tm_year = (int)(y - 1900);
	tm->tm_mon = (int)(m - 1);
	tm->tm_mday = (int)d;
	tm->tm_yday = before[m - 1] + (int)d - 1 + (leap && m > 2);
	tm->tm_isdst = 0;
	return tm;
}

struct tm *gmtime(const time_t *t)
{
	static struct tm tm;

	return gmtime_r(t, &tm);
}
