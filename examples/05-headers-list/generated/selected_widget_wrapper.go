package generated

/*
#include <stdlib.h>
#include "selected_widget_wrapper.h"
*/
import "C"

import "errors"

type SelectedWidget struct {
    ptr *C.SelectedWidgetHandle
    owned bool
    root *bool
}

func NewSelectedWidget() (*SelectedWidget, error) {
    ptr := C.cgowrap_SelectedWidget_new()
    if ptr == nil {
        return nil, errors.New("wrapper returned nil facade handle")
    }
    return newOwnedSelectedWidget(ptr), nil
}

func (s *SelectedWidget) Close() {
    if s == nil || s.ptr == nil {
        return
    }
    if !s.owned {
        return
    }
    if s.root != nil {
        *s.root = true
    }
    C.cgowrap_SelectedWidget_delete(s.ptr)
    s.ptr = nil
}

func newOwnedSelectedWidget(ptr *C.SelectedWidgetHandle) *SelectedWidget {
    if ptr == nil {
        return nil
    }
    root := new(bool)
    return &SelectedWidget{ptr: ptr, owned: true, root: root}
}

func newBorrowedSelectedWidget(ptr *C.SelectedWidgetHandle, root *bool) *SelectedWidget {
    if ptr == nil {
        return nil
    }
    return &SelectedWidget{ptr: ptr, root: root}
}

func requireSelectedWidgetHandle(s *SelectedWidget) *C.SelectedWidgetHandle {
    if s == nil || s.ptr == nil {
        panic("SelectedWidget handle is required but nil")
    }
    if s.root != nil && *s.root {
        panic("SelectedWidget handle is closed")
    }
    return s.ptr
}

func optionalSelectedWidgetHandle(s *SelectedWidget) *C.SelectedWidgetHandle {
    if s == nil {
        return nil
    }
    if s.root != nil && *s.root {
        panic("SelectedWidget handle is closed")
    }
    return s.ptr
}

func (s *SelectedWidget) Value() int32 {
    if s == nil || s.ptr == nil {
        return 0
    }
    if s.root != nil && *s.root {
        panic("SelectedWidget handle is closed")
    }
    return int32(C.cgowrap_SelectedWidget_Value(s.ptr))
}
