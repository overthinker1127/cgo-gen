#ifndef CGOWRAP_DEFAULT_ARGUMENTS_WRAPPER_H
#define CGOWRAP_DEFAULT_ARGUMENTS_WRAPPER_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <stdlib.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct DefaultCounterHandle DefaultCounterHandle;

DefaultCounterHandle* cgowrap_DefaultCounter_new__int_int(int start, int step);
DefaultCounterHandle* cgowrap_DefaultCounter_new__int(int start);
DefaultCounterHandle* cgowrap_DefaultCounter_new__void(void);
void cgowrap_DefaultCounter_delete(DefaultCounterHandle* self);
int cgowrap_DefaultCounter_Value(const DefaultCounterHandle* self);
int cgowrap_DefaultCounter_Add__int_int_mut(DefaultCounterHandle* self, int value, int multiplier);
int cgowrap_DefaultCounter_Add__int_mut(DefaultCounterHandle* self, int value);
int cgowrap_Clamp__int_int_int(int value, int min, int max);
int cgowrap_Clamp__int_int(int value, int min);
int cgowrap_Clamp__int(int value);
#ifdef __cplusplus
}
#endif

#endif /* CGOWRAP_DEFAULT_ARGUMENTS_WRAPPER_H */
