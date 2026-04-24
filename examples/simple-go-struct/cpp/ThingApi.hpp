#pragma once

#include <stdint.h>

#include "ThingModel.hpp"

class ThingApi {
public:
    ThingApi();
    ~ThingApi();

    bool SelectThing(int32_t id, ThingModel& out) const;
    bool NextThing(int32_t& pos, ThingModel& out) const;
};
