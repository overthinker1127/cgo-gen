#ifndef CGOWRAP_SCORE_WRAPPER_H
#define CGOWRAP_SCORE_WRAPPER_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <stdlib.h>

#ifdef __cplusplus
extern "C" {
#endif

int cgowrap_score_delta(int current, int previous);
int cgowrap_score_total(int wins, int draws);
#ifdef __cplusplus
}
#endif

#endif /* CGOWRAP_SCORE_WRAPPER_H */
