#pragma once
#include <stddef.h>
#include <stdarg.h>

typedef struct FILE FILE;

extern FILE *stdin;
extern FILE *stdout;
extern FILE *stderr;

#define EOF (-1)

FILE  *fopen(const char *path, const char *mode);
int    fclose(FILE *fp);
size_t fread(void *ptr, size_t size, size_t nmemb, FILE *fp);
size_t fwrite(const void *ptr, size_t size, size_t nmemb, FILE *fp);
int    fseek(FILE *fp, long offset, int whence);
long   ftell(FILE *fp);
int    fflush(FILE *fp);
char  *fgets(char *s, int size, FILE *fp);
int    fputs(const char *s, FILE *fp);
int    feof(FILE *fp);
int    ferror(FILE *fp);

int    printf(const char *fmt, ...);
int    fprintf(FILE *fp, const char *fmt, ...);
int    sprintf(char *buf, const char *fmt, ...);
int    snprintf(char *buf, size_t size, const char *fmt, ...);
int    vsnprintf(char *buf, size_t size, const char *fmt, va_list ap);
int    vfprintf(FILE *fp, const char *fmt, va_list ap);
void write(int fd, const char* buf, unsigned long count);
void exit(int code);
