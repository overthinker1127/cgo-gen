#ifndef CGOWRAP_DEVICE_CONTROLLER_WRAPPER_H
#define CGOWRAP_DEVICE_CONTROLLER_WRAPPER_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <stdlib.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct DeviceControllerHandle DeviceControllerHandle;

DeviceControllerHandle* cgowrap_DeviceController_new(void);
void cgowrap_DeviceController_delete(DeviceControllerHandle* self);
int64_t cgowrap_DeviceController_State(const DeviceControllerHandle* self);
void cgowrap_DeviceController_SetState(DeviceControllerHandle* self, int64_t state);
int64_t cgowrap_DeviceController_Mode(const DeviceControllerHandle* self);
bool cgowrap_DeviceController_SetMode(DeviceControllerHandle* self, int64_t mode);
void cgowrap_DeviceController_EnableFeature(DeviceControllerHandle* self, int feature);
bool cgowrap_DeviceController_IsFeatureEnabled(const DeviceControllerHandle* self, int feature);
#ifdef __cplusplus
}
#endif

#endif /* CGOWRAP_DEVICE_CONTROLLER_WRAPPER_H */
