#include "selected_widget_wrapper.h"
#include <cstdlib>
#include <cstring>
#include <new>
#include <string>

#include "shared_dependency.hpp"
#include "selected_widget.hpp"

SelectedWidgetHandle* cgowrap_SelectedWidget_new(void) {
    return reinterpret_cast<SelectedWidgetHandle*>(new SelectedWidget());
}

void cgowrap_SelectedWidget_delete(SelectedWidgetHandle* self) {
    delete reinterpret_cast<SelectedWidget*>(self);
}

int cgowrap_SelectedWidget_Value(const SelectedWidgetHandle* self) {
    return reinterpret_cast<const SelectedWidget*>(self)->Value();
}
