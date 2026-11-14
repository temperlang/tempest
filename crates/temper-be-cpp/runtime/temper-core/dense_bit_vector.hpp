#pragma once
#include <cstddef>
#include <cstdint>
#include <memory>
#include <vector>
#include "temper_bubble.hpp"
#include "base_types.hpp"

namespace temper {
    namespace core {
        namespace DenseBitVector {

            inline std::shared_ptr<std::vector<bool>> make(int32_t size) {
                return std::make_shared<std::vector<bool>>(size, false);
            }

            inline bool get(std::shared_ptr<std::vector<bool>> bv, int32_t index) {
                // A bit never set is false, as core's Temper body has it.
                int32_t sz = static_cast<int32_t>(bv->size());
                return 0 <= index && index < sz && (*bv)[index];
            }

            inline void set(std::shared_ptr<std::vector<bool>> bv, int32_t index, bool value) {
                // As core's Temper body: past the end, a false bit is
                // already false, and a true one grows the vector.
                int32_t sz = static_cast<int32_t>(bv->size());
                if (index < 0) {
                    bubble("DenseBitVector set index out of bounds");
                }
                if (index >= sz) {
                    if (!value) {
                        return;
                    }
                    bv->resize(static_cast<std::size_t>(index) + 1, false);
                }
                (*bv)[index] = value;
            }

        }
    }
}
