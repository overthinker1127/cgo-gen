#include "sensor_reading_wrapper.h"
#include <cstdlib>
#include <cstring>
#include <new>
#include <string>

#include "sensor_reading.hpp"

SensorReadingHandle* cgowrap_SensorReading_new(void) {
    return reinterpret_cast<SensorReadingHandle*>(new SensorReading());
}

void cgowrap_SensorReading_delete(SensorReadingHandle* self) {
    delete reinterpret_cast<SensorReading*>(self);
}

int cgowrap_SensorReading_GetSampleId(const SensorReadingHandle* self) {
    return reinterpret_cast<const SensorReading*>(self)->sample_id;
}

void cgowrap_SensorReading_SetSampleId(SensorReadingHandle* self, int value) {
    reinterpret_cast<SensorReading*>(self)->sample_id = value;
}

double cgowrap_SensorReading_GetTemperatureC(const SensorReadingHandle* self) {
    return reinterpret_cast<const SensorReading*>(self)->temperature_c;
}

void cgowrap_SensorReading_SetTemperatureC(SensorReadingHandle* self, double value) {
    reinterpret_cast<SensorReading*>(self)->temperature_c = value;
}

const char* cgowrap_SensorReading_GetLabel(const SensorReadingHandle* self) {
    return reinterpret_cast<const SensorReading*>(self)->label;
}

void cgowrap_SensorReading_SetLabel(SensorReadingHandle* self, const char* value) {
    if (value == nullptr) {
        reinterpret_cast<SensorReading*>(self)->label[0] = '\0';
        return;
    }
    std::strncpy(reinterpret_cast<SensorReading*>(self)->label, value, 31);
    reinterpret_cast<SensorReading*>(self)->label[31] = '\0';
}

int cgowrap_SensorReading_GetSchemaVersion(const SensorReadingHandle* self) {
    return reinterpret_cast<const SensorReading*>(self)->schema_version;
}
