#pragma once
// Minimal JSON value type: just enough to parse/dump the catalog file format
// (an object of arbitrary, opaque item dicts keyed by "name"). Not a general-
// purpose JSON library - if the native app needs richer JSON later, swap this
// for a real dependency (nlohmann/json) via CMake FetchContent.

#include <cctype>
#include <map>
#include <memory>
#include <sstream>
#include <stdexcept>
#include <string>
#include <variant>
#include <vector>

namespace omnipack {

class JsonValue {
public:
    enum class Type { Null, Bool, Number, String, Array, Object };

    JsonValue() : type_(Type::Null) {}
    static JsonValue make_object() { JsonValue v; v.type_ = Type::Object; return v; }
    static JsonValue make_array() { JsonValue v; v.type_ = Type::Array; return v; }
    static JsonValue make_string(std::string s) { JsonValue v; v.type_ = Type::String; v.str_ = std::move(s); return v; }
    static JsonValue make_number(double n) { JsonValue v; v.type_ = Type::Number; v.num_ = n; return v; }
    static JsonValue make_bool(bool b) { JsonValue v; v.type_ = Type::Bool; v.bool_ = b; return v; }

    Type type() const { return type_; }
    bool is_object() const { return type_ == Type::Object; }
    bool is_array() const { return type_ == Type::Array; }
    bool is_string() const { return type_ == Type::String; }

    const std::string& as_string() const { return str_; }
    double as_number() const { return num_; }
    bool as_bool() const { return bool_; }

    std::vector<JsonValue>& array() { type_ = Type::Array; return arr_; }
    const std::vector<JsonValue>& array() const { return arr_; }

    std::map<std::string, JsonValue>& object() { type_ = Type::Object; return obj_; }
    const std::map<std::string, JsonValue>& object() const { return obj_; }

    bool has(const std::string& key) const { return obj_.find(key) != obj_.end(); }
    const JsonValue& at(const std::string& key) const { return obj_.at(key); }
    JsonValue& operator[](const std::string& key) { type_ = Type::Object; return obj_[key]; }

    // --- Parsing ---
    static JsonValue parse(const std::string& text) {
        size_t pos = 0;
        skip_ws(text, pos);
        JsonValue v = parse_value(text, pos);
        return v;
    }

    // --- Serialization ---
    std::string dump(int indent = 4) const {
        std::ostringstream out;
        dump_impl(out, indent, 0);
        return out.str();
    }

private:
    Type type_ = Type::Null;
    std::string str_;
    double num_ = 0.0;
    bool bool_ = false;
    std::vector<JsonValue> arr_;
    std::map<std::string, JsonValue> obj_;

    static void skip_ws(const std::string& s, size_t& pos) {
        while (pos < s.size() && std::isspace(static_cast<unsigned char>(s[pos]))) ++pos;
    }

    static JsonValue parse_value(const std::string& s, size_t& pos) {
        skip_ws(s, pos);
        if (pos >= s.size()) throw std::runtime_error("Unexpected end of JSON input");
        char c = s[pos];
        if (c == '{') return parse_object(s, pos);
        if (c == '[') return parse_array(s, pos);
        if (c == '"') return make_string(parse_string(s, pos));
        if (c == 't' || c == 'f') return parse_bool(s, pos);
        if (c == 'n') { pos += 4; return JsonValue(); } // "null"
        return parse_number(s, pos);
    }

    static JsonValue parse_object(const std::string& s, size_t& pos) {
        JsonValue v = make_object();
        ++pos; // '{'
        skip_ws(s, pos);
        if (pos < s.size() && s[pos] == '}') { ++pos; return v; }
        while (true) {
            skip_ws(s, pos);
            std::string key = parse_string(s, pos);
            skip_ws(s, pos);
            if (s[pos] != ':') throw std::runtime_error("Expected ':' in JSON object");
            ++pos;
            v.obj_[key] = parse_value(s, pos);
            skip_ws(s, pos);
            if (pos < s.size() && s[pos] == ',') { ++pos; continue; }
            if (pos < s.size() && s[pos] == '}') { ++pos; break; }
            throw std::runtime_error("Expected ',' or '}' in JSON object");
        }
        return v;
    }

    static JsonValue parse_array(const std::string& s, size_t& pos) {
        JsonValue v = make_array();
        ++pos; // '['
        skip_ws(s, pos);
        if (pos < s.size() && s[pos] == ']') { ++pos; return v; }
        while (true) {
            v.arr_.push_back(parse_value(s, pos));
            skip_ws(s, pos);
            if (pos < s.size() && s[pos] == ',') { ++pos; continue; }
            if (pos < s.size() && s[pos] == ']') { ++pos; break; }
            throw std::runtime_error("Expected ',' or ']' in JSON array");
        }
        return v;
    }

    static std::string parse_string(const std::string& s, size_t& pos) {
        if (s[pos] != '"') throw std::runtime_error("Expected '\"' to start JSON string");
        ++pos;
        std::string out;
        while (pos < s.size() && s[pos] != '"') {
            char c = s[pos];
            if (c == '\\' && pos + 1 < s.size()) {
                char next = s[pos + 1];
                switch (next) {
                    case 'n': out += '\n'; break;
                    case 't': out += '\t'; break;
                    case 'r': out += '\r'; break;
                    case '"': out += '"'; break;
                    case '\\': out += '\\'; break;
                    case '/': out += '/'; break;
                    default: out += next; break;
                }
                pos += 2;
            } else {
                out += c;
                ++pos;
            }
        }
        ++pos; // closing '"'
        return out;
    }

    static JsonValue parse_bool(const std::string& s, size_t& pos) {
        if (s.compare(pos, 4, "true") == 0) { pos += 4; return make_bool(true); }
        pos += 5; // "false"
        return make_bool(false);
    }

    static JsonValue parse_number(const std::string& s, size_t& pos) {
        size_t start = pos;
        if (pos < s.size() && (s[pos] == '-' || s[pos] == '+')) ++pos;
        while (pos < s.size() && (std::isdigit(static_cast<unsigned char>(s[pos])) || s[pos] == '.' ||
                                   s[pos] == 'e' || s[pos] == 'E' || s[pos] == '+' || s[pos] == '-')) {
            ++pos;
        }
        return make_number(std::stod(s.substr(start, pos - start)));
    }

    static void escape_into(std::ostringstream& out, const std::string& s) {
        for (char c : s) {
            switch (c) {
                case '"': out << "\\\""; break;
                case '\\': out << "\\\\"; break;
                case '\n': out << "\\n"; break;
                case '\t': out << "\\t"; break;
                case '\r': out << "\\r"; break;
                default: out << c;
            }
        }
    }

    void dump_impl(std::ostringstream& out, int indent, int depth) const {
        std::string pad(indent * (depth + 1), ' ');
        std::string pad_close(indent * depth, ' ');
        switch (type_) {
            case Type::Null: out << "null"; break;
            case Type::Bool: out << (bool_ ? "true" : "false"); break;
            case Type::Number: {
                if (num_ == static_cast<long long>(num_)) out << static_cast<long long>(num_);
                else out << num_;
                break;
            }
            case Type::String: out << '"'; escape_into(out, str_); out << '"'; break;
            case Type::Array: {
                if (arr_.empty()) { out << "[]"; break; }
                out << "[\n";
                for (size_t i = 0; i < arr_.size(); ++i) {
                    out << pad;
                    arr_[i].dump_impl(out, indent, depth + 1);
                    if (i + 1 < arr_.size()) out << ",";
                    out << "\n";
                }
                out << pad_close << "]";
                break;
            }
            case Type::Object: {
                if (obj_.empty()) { out << "{}"; break; }
                out << "{\n";
                size_t i = 0;
                for (const auto& [key, val] : obj_) {
                    out << pad << '"'; escape_into(out, key); out << "\": ";
                    val.dump_impl(out, indent, depth + 1);
                    if (++i < obj_.size()) out << ",";
                    out << "\n";
                }
                out << pad_close << "}";
                break;
            }
        }
    }
};

} // namespace omnipack
