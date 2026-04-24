#include "ThingApi.hpp"

#include <string>

namespace {

void fillThing(int32_t slot, ThingModel& out) {
    out.SetValue(slot * 10);
    std::string name = "item-" + std::to_string(slot);
    out.SetName(name.c_str());
}

} // namespace

ThingApi::ThingApi() = default;

ThingApi::~ThingApi() = default;

bool ThingApi::SelectThing(int32_t id, ThingModel& out) const {
    if (id < 0 || id > 2) {
        return false;
    }

    fillThing(id, out);
    return true;
}

bool ThingApi::NextThing(int32_t& pos, ThingModel& out) const {
    if (pos < 0) {
        pos = 0;
    }
    if (pos > 2) {
        return false;
    }

    fillThing(pos, out);
    ++pos;
    return true;
}
