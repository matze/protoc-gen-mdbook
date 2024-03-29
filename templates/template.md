{% macro enum_type(t) %}
enum {{ t.name }} {
{%- for value in t.values -%}
  {% if value.leading_comments != "" %}
  {{ value.leading_comments|render_multiline_comment|indent(2) }}
  {%- endif %}
  {{ value.name }} = {{ value.number }}; {% if value.trailing_comments != "" %} // {{- value.trailing_comments -}} {%- endif -%}
{%- endfor %}
}
{% endmacro %}

{% macro render_type(t) %}
{%- match t -%}
  {%- when proto::Types::Message with (t) -%}
    {{ t.description }}
  {%- when proto::Types::Enum with (t) -%}
    {{ t.description }}
  {%- else -%}
{%- endmatch -%}

```protobuf
{%- match t -%}
  {%- when proto::Types::Message with (t) -%}
    {{ t.render().unwrap() }}
  {%- when proto::Types::Enum with (t) -%}
    {%- call enum_type(t) -%}
  {%- else -%}
{%- endmatch -%}
```
{% endmacro %}

{% for service in services %}
## {{ service.package }}.{{ service.name }}

{% if service.deprecated -%}
<kbd>deprecated</kbd>
{%- endif %}

{{ service.description }}

{% if service.methods.len() > 2 %}
### Methods

{% for method in service.methods %}
<a href="#{{ method.name|lower }}">`{{ method.name }}()`</a>
{% endfor %}
{% endif %}

{% if !service.deprecated_methods.is_empty() %}
#### Deprecated
{% for method in service.deprecated_methods %}
<a href="#{{ method.name|lower }}">`{{ method.name }}()`</a>
{% endfor %}
{% endif %}

{% for method in service.methods %}
{% if options.optimize_for_doxygen %}
### {{ method.name }}()  {{ "{{#{}}}"|format(method.name|lower) }}
{% else %}
### `{{ method.name }}()`
{% endif %}

<kbd>{{ method.call_type }}</kbd>{% if method.deprecated %} <kbd>deprecated</kbd>{% endif %}

{{ method.description }}

**Input**

{% for t in method.input_types %}
{%- call render_type(t) -%}
{% endfor %}

**Output**

{% for t in method.output_types %}
{%- call render_type(t) -%}
{% endfor %}
{% endfor %}

{% for method in service.deprecated_methods %}
{% if options.optimize_for_doxygen %}
### {{ method.name }}()  {{ "{{#{}}}"|format(method.name|lower) }}
{% else %}
### `{{ method.name }}()`
{% endif %}

<kbd>{{ method.call_type }}</kbd>{% if method.deprecated %} <kbd>deprecated</kbd>{% endif %}

{{ method.description }}

**Input**

{% for t in method.input_types %}
{%- call render_type(t) -%}
{% endfor %}

**Output**

{% for t in method.output_types %}
{%- call render_type(t) -%}
{% endfor %}
{% endfor %}

{% endfor %}
