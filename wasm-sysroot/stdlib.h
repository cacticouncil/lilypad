#define NULL ((void*)0)

void* malloc(unsigned long size);
void* calloc(unsigned long nmemb, unsigned long size);
void free(void* ptr);
void* realloc(void* ptr, unsigned long size);
