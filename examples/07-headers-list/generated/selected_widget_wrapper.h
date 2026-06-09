#ifndef CGOWRAP_SELECTED_WIDGET_WRAPPER_H
#define CGOWRAP_SELECTED_WIDGET_WRAPPER_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <stdlib.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct SelectedWidgetHandle SelectedWidgetHandle;

SelectedWidgetHandle* cgowrap_SelectedWidget_new(void);
void cgowrap_SelectedWidget_delete(SelectedWidgetHandle* self);
int cgowrap_SelectedWidget_Value(const SelectedWidgetHandle* self);
#ifdef __cplusplus
}
#endif

#endif /* CGOWRAP_SELECTED_WIDGET_WRAPPER_H */
