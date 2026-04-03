#pragma once
#include <stddef.h>

void  *malloc(size_t size);
void  *calloc(size_t nmemb, size_t size);
void  *realloc(void *ptr, size_t size);
void   free(void *ptr);

void   exit(int status);
void   abort(void);

int    atoi(const char *s);
long   atol(const char *s);
double atof(const char *s);
long   strtol(const char *s, char **endptr, int base);
double strtod(const char *s, char **endptr);

char  *getenv(const char *name);
void   qsort(void *base, size_t nmemb, size_t size, int (*cmp)(const void *, const void *));
int    abs(int x);
