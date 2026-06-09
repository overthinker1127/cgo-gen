#ifndef CGOWRAP_A_WRAPPER_H
#define CGOWRAP_A_WRAPPER_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <stdlib.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct AHandle AHandle;
typedef struct BHandle BHandle;

AHandle* cgowrap_A_new(void);
void cgowrap_A_delete(AHandle* self);
BHandle* cgowrap_A_Child(AHandle* self);
int cgowrap_A_ChildValue(const AHandle* self);
void cgowrap_A_SetChildValue(AHandle* self, int value);
#ifdef __cplusplus
}
#endif

#endif /* CGOWRAP_A_WRAPPER_H */
