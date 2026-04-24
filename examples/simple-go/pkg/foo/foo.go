package foo

/*
#cgo CXXFLAGS: -std=c++17 -I${SRCDIR} -I${SRCDIR}/../../../simple-cpp/include
#cgo LDFLAGS: -lstdc++

#include <stdlib.h>
#include "foo_wrapper.h"
*/
import "C"

import (
	"errors"
)

type Bar struct {
	ptr *C.fooBarHandle
}

func NewBar(value int32) *Bar {
	ptr := C.cgowrap_foo_Bar_new(C.int(value))
	return &Bar{ptr: ptr}
}

func (b *Bar) Close() {
	if b == nil || b.ptr == nil {
		return
	}
	C.cgowrap_foo_Bar_delete(b.ptr)
	b.ptr = nil
}

func (b *Bar) Value() int32 {
	return int32(C.cgowrap_foo_Bar_value(b.ptr))
}

func (b *Bar) SetValue(value int32) {
	C.cgowrap_foo_Bar_set_value(b.ptr, C.int(value))
}

func (b *Bar) Name() (string, error) {
	raw := C.cgowrap_foo_Bar_name(b.ptr)
	if raw == nil {
		return "", errors.New("wrapper returned nil string")
	}
	defer C.cgowrap_string_free(raw)
	return C.GoString(raw), nil
}

func Add(lhs, rhs int32) int32 {
	return int32(C.cgowrap_foo_add(C.int(lhs), C.int(rhs)))
}
