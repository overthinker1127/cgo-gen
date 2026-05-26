package generated

/*
#include <stdlib.h>
#include "device_controller_wrapper.h"
*/
import "C"

import "errors"

type DeviceMode int64

const (
    DeviceModeManual DeviceMode = 10
    DeviceModeAutomatic DeviceMode = 20
)

type DeviceState int64

const (
    DeviceStateOffline DeviceState = 0
    DeviceStateOnline DeviceState = 1
    DeviceStateError DeviceState = 2
)

const (
    DeviceFeatureLogging = 1
    DeviceFeatureMetrics = 2
)

type DeviceController struct {
    ptr *C.DeviceControllerHandle
    owned bool
    root *bool
}

func NewDeviceController() (*DeviceController, error) {
    ptr := C.cgowrap_DeviceController_new()
    if ptr == nil {
        return nil, errors.New("wrapper returned nil facade handle")
    }
    return newOwnedDeviceController(ptr), nil
}

func (d *DeviceController) Close() {
    if d == nil || d.ptr == nil {
        return
    }
    if !d.owned {
        return
    }
    if d.root != nil {
        *d.root = true
    }
    C.cgowrap_DeviceController_delete(d.ptr)
    d.ptr = nil
}

func newOwnedDeviceController(ptr *C.DeviceControllerHandle) *DeviceController {
    if ptr == nil {
        return nil
    }
    root := new(bool)
    return &DeviceController{ptr: ptr, owned: true, root: root}
}

func newBorrowedDeviceController(ptr *C.DeviceControllerHandle, root *bool) *DeviceController {
    if ptr == nil {
        return nil
    }
    return &DeviceController{ptr: ptr, root: root}
}

func requireDeviceControllerHandle(d *DeviceController) *C.DeviceControllerHandle {
    if d == nil || d.ptr == nil {
        panic("DeviceController handle is required but nil")
    }
    if d.root != nil && *d.root {
        panic("DeviceController handle is closed")
    }
    return d.ptr
}

func optionalDeviceControllerHandle(d *DeviceController) *C.DeviceControllerHandle {
    if d == nil {
        return nil
    }
    if d.root != nil && *d.root {
        panic("DeviceController handle is closed")
    }
    return d.ptr
}

func (d *DeviceController) State() DeviceState {
    if d == nil || d.ptr == nil {
        return 0
    }
    if d.root != nil && *d.root {
        panic("DeviceController handle is closed")
    }
    return DeviceState(C.cgowrap_DeviceController_State(d.ptr))
}

func (d *DeviceController) SetState(state DeviceState) {
    if d == nil || d.ptr == nil {
        return
    }
    if d.root != nil && *d.root {
        panic("DeviceController handle is closed")
    }
    C.cgowrap_DeviceController_SetState(d.ptr, C.int64_t(state))
}

func (d *DeviceController) Mode() DeviceMode {
    if d == nil || d.ptr == nil {
        return 0
    }
    if d.root != nil && *d.root {
        panic("DeviceController handle is closed")
    }
    return DeviceMode(C.cgowrap_DeviceController_Mode(d.ptr))
}

func (d *DeviceController) SetMode(mode DeviceMode) bool {
    if d == nil || d.ptr == nil {
        return false
    }
    if d.root != nil && *d.root {
        panic("DeviceController handle is closed")
    }
    result := C.cgowrap_DeviceController_SetMode(d.ptr, C.int64_t(mode))
    return bool(result)
}

func (d *DeviceController) EnableFeature(feature int32) {
    if d == nil || d.ptr == nil {
        return
    }
    if d.root != nil && *d.root {
        panic("DeviceController handle is closed")
    }
    C.cgowrap_DeviceController_EnableFeature(d.ptr, C.int(feature))
}

func (d *DeviceController) IsFeatureEnabled(feature int32) bool {
    if d == nil || d.ptr == nil {
        return false
    }
    if d.root != nil && *d.root {
        panic("DeviceController handle is closed")
    }
    result := C.cgowrap_DeviceController_IsFeatureEnabled(d.ptr, C.int(feature))
    return bool(result)
}
