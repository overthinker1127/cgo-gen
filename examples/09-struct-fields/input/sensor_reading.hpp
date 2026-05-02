#pragma once

#include <stdint.h>

struct SensorReading {
    int32_t sample_id;
    double temperature_c;
    char label[32];
    const int32_t schema_version = 1;
};
