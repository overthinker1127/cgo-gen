#ifndef CGOWRAP_COUNTER_WRAPPER_H
#define CGOWRAP_COUNTER_WRAPPER_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <stdlib.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct metricsCounterHandle metricsCounterHandle;

metricsCounterHandle* cgowrap_metrics_Counter_new(int initial_value);
void cgowrap_metrics_Counter_delete(metricsCounterHandle* self);
int cgowrap_metrics_Counter_value(const metricsCounterHandle* self);
void cgowrap_metrics_Counter_increment(metricsCounterHandle* self, int amount);
char* cgowrap_metrics_Counter_label(const metricsCounterHandle* self);
int cgowrap_metrics_clamp_to_zero(int value);
void cgowrap_string_free(char* value);

#ifdef __cplusplus
}
#endif

#endif /* CGOWRAP_COUNTER_WRAPPER_H */
