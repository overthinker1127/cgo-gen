package generated

/*
#include <stdlib.h>
#include "sensor_reading_wrapper.h"
*/
import "C"

import "errors"

import "unsafe"

type SensorReading struct {
    ptr *C.SensorReadingHandle
    owned bool
    root *bool
}

func NewSensorReading() (*SensorReading, error) {
    ptr := C.cgowrap_SensorReading_new()
    if ptr == nil {
        return nil, errors.New("wrapper returned nil facade handle")
    }
    return newOwnedSensorReading(ptr), nil
}

func (s *SensorReading) Close() {
    if s == nil || s.ptr == nil {
        return
    }
    if !s.owned {
        return
    }
    if s.root != nil {
        *s.root = true
    }
    C.cgowrap_SensorReading_delete(s.ptr)
    s.ptr = nil
}

func newOwnedSensorReading(ptr *C.SensorReadingHandle) *SensorReading {
    if ptr == nil {
        return nil
    }
    root := new(bool)
    return &SensorReading{ptr: ptr, owned: true, root: root}
}

func newBorrowedSensorReading(ptr *C.SensorReadingHandle, root *bool) *SensorReading {
    if ptr == nil {
        return nil
    }
    return &SensorReading{ptr: ptr, root: root}
}

func requireSensorReadingHandle(s *SensorReading) *C.SensorReadingHandle {
    if s == nil || s.ptr == nil {
        panic("SensorReading handle is required but nil")
    }
    if s.root != nil && *s.root {
        panic("SensorReading handle is closed")
    }
    return s.ptr
}

func optionalSensorReadingHandle(s *SensorReading) *C.SensorReadingHandle {
    if s == nil {
        return nil
    }
    if s.root != nil && *s.root {
        panic("SensorReading handle is closed")
    }
    return s.ptr
}

func (s *SensorReading) GetSampleId() int32 {
    if s == nil || s.ptr == nil {
        return 0
    }
    if s.root != nil && *s.root {
        panic("SensorReading handle is closed")
    }
    return int32(C.cgowrap_SensorReading_GetSampleId(s.ptr))
}

func (s *SensorReading) SetSampleId(value int32) {
    if s == nil || s.ptr == nil {
        return
    }
    if s.root != nil && *s.root {
        panic("SensorReading handle is closed")
    }
    C.cgowrap_SensorReading_SetSampleId(s.ptr, C.int32_t(value))
}

func (s *SensorReading) GetTemperatureC() float64 {
    if s == nil || s.ptr == nil {
        return 0
    }
    if s.root != nil && *s.root {
        panic("SensorReading handle is closed")
    }
    return float64(C.cgowrap_SensorReading_GetTemperatureC(s.ptr))
}

func (s *SensorReading) SetTemperatureC(value float64) {
    if s == nil || s.ptr == nil {
        return
    }
    if s.root != nil && *s.root {
        panic("SensorReading handle is closed")
    }
    C.cgowrap_SensorReading_SetTemperatureC(s.ptr, C.double(value))
}

func (s *SensorReading) GetLabel() (string, error) {
    if s == nil || s.ptr == nil {
        return "", errors.New("facade receiver is nil")
    }
    if s.root != nil && *s.root {
        panic("SensorReading handle is closed")
    }
    raw := C.cgowrap_SensorReading_GetLabel(s.ptr)
    if raw == nil {
        return "", errors.New("wrapper returned nil string")
    }
    return C.GoString(raw), nil
}

func (s *SensorReading) SetLabel(value string) {
    if s == nil || s.ptr == nil {
        return
    }
    if s.root != nil && *s.root {
        panic("SensorReading handle is closed")
    }
    cArg0 := C.CString(value)
    defer C.free(unsafe.Pointer(cArg0))
    C.cgowrap_SensorReading_SetLabel(s.ptr, cArg0)
}

func (s *SensorReading) GetSchemaVersion() int32 {
    if s == nil || s.ptr == nil {
        return 0
    }
    if s.root != nil && *s.root {
        panic("SensorReading handle is closed")
    }
    return int32(C.cgowrap_SensorReading_GetSchemaVersion(s.ptr))
}
