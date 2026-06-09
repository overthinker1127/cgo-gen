#ifndef CGOWRAP_SELECTED_COUNTER_WRAPPER_H
#define CGOWRAP_SELECTED_COUNTER_WRAPPER_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <stdlib.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct SelectedCounterHandle SelectedCounterHandle;

SelectedCounterHandle* cgowrap_SelectedCounter_new(void);
void cgowrap_SelectedCounter_delete(SelectedCounterHandle* self);
int cgowrap_SelectedCounter_Increment(const SelectedCounterHandle* self, int value);
#ifdef __cplusplus
}
#endif

#endif /* CGOWRAP_SELECTED_COUNTER_WRAPPER_H */
