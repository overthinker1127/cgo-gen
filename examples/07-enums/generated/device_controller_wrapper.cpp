#include "device_controller_wrapper.h"
#include <cstdlib>
#include <cstring>
#include <new>
#include <string>

#include "device_controller.hpp"

DeviceControllerHandle* cgowrap_DeviceController_new(void) {
    return reinterpret_cast<DeviceControllerHandle*>(new DeviceController());
}

void cgowrap_DeviceController_delete(DeviceControllerHandle* self) {
    delete reinterpret_cast<DeviceController*>(self);
}

int64_t cgowrap_DeviceController_State(const DeviceControllerHandle* self) {
    return static_cast<int64_t>(reinterpret_cast<const DeviceController*>(self)->State());
}

void cgowrap_DeviceController_SetState(DeviceControllerHandle* self, int64_t state) {
    reinterpret_cast<DeviceController*>(self)->SetState(static_cast<DeviceState>(state));
}

int64_t cgowrap_DeviceController_Mode(const DeviceControllerHandle* self) {
    return static_cast<int64_t>(reinterpret_cast<const DeviceController*>(self)->Mode());
}

bool cgowrap_DeviceController_SetMode(DeviceControllerHandle* self, int64_t mode) {
    return reinterpret_cast<DeviceController*>(self)->SetMode(static_cast<DeviceMode>(mode));
}

void cgowrap_DeviceController_EnableFeature(DeviceControllerHandle* self, int feature) {
    reinterpret_cast<DeviceController*>(self)->EnableFeature(feature);
}

bool cgowrap_DeviceController_IsFeatureEnabled(const DeviceControllerHandle* self, int feature) {
    return reinterpret_cast<const DeviceController*>(self)->IsFeatureEnabled(feature);
}
