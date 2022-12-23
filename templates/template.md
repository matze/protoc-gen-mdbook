{% macro message_type(t) %}
{%- if t.fields.is_empty() %}
message {{ t.name }} {}
{% else %}
message {{ t.name }} {
{%- for field in t.fields %}
  {% if field.leading_comments != "" -%}
  {{ field.leading_comments|render_multiline_comment|indent(2) }}
  {%- endif %}
  {% if field.optional %}optional {% endif %}{{ field.typ.name() }} {{ field.name }} = {{ field.number }}; {% if field.trailing_comments != "" %} // {{- field.trailing_comments -}}
  {% endif %}
{%- endfor %}
}
{% endif -%}
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

{% for method in service.methods %}
### `{{ method.name }}()`

<kbd>{{ method.call_type }}</kbd>{% if method.deprecated %} <kbd>deprecated</kbd>{% endif %}

{{ method.description }}

**Input**

{% for t in method.input_types %}
{{ t.description }}

```protobuf
{%- call message_type(t) -%}
```
{% endfor %}

**Output**

{% for t in method.output_types %}
{{ t.description }}

```protobuf
{%- call message_type(t) -%}
```
{% endfor %}

{% endfor %}
{% endfor %}
