#pragma once
// Direct port of the catalog CRUD in src/omnipack/logic.py:
// get_catalog_data / save_to_catalog_data / delete_from_catalog_data.
// Catalog entries are opaque JSON objects; only the "name" field is
// inspected, for upsert/delete matching.

#include <filesystem>
#include <fstream>
#include <sstream>
#include <string>

#include "omnipack/json_value.hpp"

namespace omnipack {

class Catalog {
public:
    explicit Catalog(std::filesystem::path path) : path_(std::move(path)) {}

    JsonValue get_all() const {
        JsonValue result = JsonValue::make_object();
        result["items"] = JsonValue::make_array();

        if (!std::filesystem::exists(path_)) return result;

        std::ifstream in(path_);
        std::ostringstream buf;
        buf << in.rdbuf();
        try {
            return JsonValue::parse(buf.str());
        } catch (const std::exception&) {
            return result; // matches logic.py's except: return {"items": []}
        }
    }

    void save(const JsonValue& entry) {
        JsonValue catalog = get_all();
        auto& items = catalog["items"].array();
        const std::string& name = entry.at("name").as_string();

        bool replaced = false;
        for (auto& existing : items) {
            if (existing.has("name") && existing.at("name").as_string() == name) {
                existing = entry;
                replaced = true;
                break;
            }
        }
        if (!replaced) items.push_back(entry);

        write(catalog);
    }

    void remove(const std::string& name) {
        JsonValue catalog = get_all();
        auto& items = catalog["items"].array();
        std::vector<JsonValue> kept;
        for (auto& existing : items) {
            if (!(existing.has("name") && existing.at("name").as_string() == name)) kept.push_back(existing);
        }
        items = kept;
        write(catalog);
    }

private:
    void write(const JsonValue& catalog) const {
        std::ofstream out(path_);
        out << catalog.dump(4);
    }

    std::filesystem::path path_;
};

} // namespace omnipack
