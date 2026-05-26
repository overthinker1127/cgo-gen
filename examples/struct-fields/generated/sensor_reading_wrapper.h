#ifndef CGOWRAP_SENSOR_READING_WRAPPER_H
#define CGOWRAP_SENSOR_READING_WRAPPER_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <stdlib.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct SensorReadingHandle SensorReadingHandle;

SensorReadingHandle* cgowrap_SensorReading_new(void);
void cgowrap_SensorReading_delete(SensorReadingHandle* self);
int cgowrap_SensorReading_GetSampleId(const SensorReadingHandle* self);
void cgowrap_SensorReading_SetSampleId(SensorReadingHandle* self, int value);
double cgowrap_SensorReading_GetTemperatureC(const SensorReadingHandle* self);
void cgowrap_SensorReading_SetTemperatureC(SensorReadingHandle* self, double value);
const char* cgowrap_SensorReading_GetLabel(const SensorReadingHandle* self);
void cgowrap_SensorReading_SetLabel(SensorReadingHandle* self, const char* value);
int cgowrap_SensorReading_GetSchemaVersion(const SensorReadingHandle* self);
#ifdef __cplusplus
}
#endif

#endif /* CGOWRAP_SENSOR_READING_WRAPPER_H */
