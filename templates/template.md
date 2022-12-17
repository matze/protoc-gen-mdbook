{% macro message_type(t) %}
{%- if t.fields.is_empty() %}
message {{ t.name }} {}
{% else %}
message {{ t.name }} {
{%- for field in t.fields %}
  {% if field.optional %}optional {% endif %}{{ field.type_name }} {{ field.name }} = {{ field.number }};
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

{% for method in service.methods %}
### `{{ method.name }}()`

<kbd>{{ method.call_type }}</kbd>
{%- if method.deprecated -%}
<kbd>deprecated</kbd>
{%- endif %}

{{ method.description }}

**Input**

{{ method.input_type.description }}

```protobuf
{%- call message_type(method.input_type) -%}
```

**Output**

{{ method.output_type.description }}

```protobuf
{%- call message_type(method.output_type) -%}
```

{% endfor %}
{% endfor %}
