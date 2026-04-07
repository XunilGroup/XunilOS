#pragma once
#include <sys/types.h>
#include <stddef.h>
#include <stdarg.h>

typedef struct FILE FILE;

extern FILE *stdin;
extern FILE *stdout;
extern FILE *stderr;

#define EOF (-1)

#define SEEK_SET 0
#define SEEK_CUR 1
#define SEEK_END 2

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
int    remove(const char *path);
int    rename(const char *path, const char *new_path);

int    printf(const char *fmt, ...);
int    puts(const char *s);
int    putchar(int c);

int    fprintf(FILE *fp, const char *fmt, ...);
int    sprintf(char *buf, const char *fmt, ...);
int    snprintf(char *buf, size_t size, const char *fmt, ...);
int    vsnprintf(char *buf, size_t size, const char *fmt, va_list ap);
int    vfprintf(FILE *fp, const char *fmt, va_list ap);
ssize_t write(int fd, const void *buf, size_t count);
void exit(int code);

int    sscanf(const char *str, const char *format, ...);
