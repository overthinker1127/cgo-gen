package generated

/*
#include <stdlib.h>
#include "counter_wrapper.h"
*/
import "C"

import "errors"

func ClampToZero(value int32) int32 {
    return int32(C.cgowrap_metrics_clamp_to_zero(C.int(value)))
}

type MetricsCounter struct {
    ptr *C.metricsCounterHandle
    owned bool
    root *bool
}

func NewMetricsCounter(initial_value int32) (*MetricsCounter, error) {
    ptr := C.cgowrap_metrics_Counter_new(C.int(initial_value))
    if ptr == nil {
        return nil, errors.New("wrapper returned nil facade handle")
    }
    return newOwnedMetricsCounter(ptr), nil
}

func (m *MetricsCounter) Close() {
    if m == nil || m.ptr == nil {
        return
    }
    if !m.owned {
        return
    }
    if m.root != nil {
        *m.root = true
    }
    C.cgowrap_metrics_Counter_delete(m.ptr)
    m.ptr = nil
}

func newOwnedMetricsCounter(ptr *C.metricsCounterHandle) *MetricsCounter {
    if ptr == nil {
        return nil
    }
    root := new(bool)
    return &MetricsCounter{ptr: ptr, owned: true, root: root}
}

func newBorrowedMetricsCounter(ptr *C.metricsCounterHandle, root *bool) *MetricsCounter {
    if ptr == nil {
        return nil
    }
    return &MetricsCounter{ptr: ptr, root: root}
}

func requireMetricsCounterHandle(m *MetricsCounter) *C.metricsCounterHandle {
    if m == nil || m.ptr == nil {
        panic("MetricsCounter handle is required but nil")
    }
    if m.root != nil && *m.root {
        panic("MetricsCounter handle is closed")
    }
    return m.ptr
}

func optionalMetricsCounterHandle(m *MetricsCounter) *C.metricsCounterHandle {
    if m == nil {
        return nil
    }
    if m.root != nil && *m.root {
        panic("MetricsCounter handle is closed")
    }
    return m.ptr
}

func (m *MetricsCounter) Value() int32 {
    if m == nil || m.ptr == nil {
        return 0
    }
    if m.root != nil && *m.root {
        panic("MetricsCounter handle is closed")
    }
    return int32(C.cgowrap_metrics_Counter_value(m.ptr))
}

func (m *MetricsCounter) Increment(amount int32) {
    if m == nil || m.ptr == nil {
        return
    }
    if m.root != nil && *m.root {
        panic("MetricsCounter handle is closed")
    }
    C.cgowrap_metrics_Counter_increment(m.ptr, C.int(amount))
}

func (m *MetricsCounter) Label() (string, error) {
    if m == nil || m.ptr == nil {
        return "", errors.New("facade receiver is nil")
    }
    if m.root != nil && *m.root {
        panic("MetricsCounter handle is closed")
    }
    raw := C.cgowrap_metrics_Counter_label(m.ptr)
    if raw == nil {
        return "", errors.New("wrapper returned nil string")
    }
    defer C.cgowrap_string_free(raw)
    return C.GoString(raw), nil
}
