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

```proto
{%- if method.input_type.fields.is_empty() %}
message {{ method.input_type.name }} {}
{% else %}
message {{ method.input_type.name }} {
{%- for field in method.input_type.fields %}
  {{ field.type_name }} {{ field.name }} = {{ field.number }};
{%- endfor %}
}
{% endif -%}
```

**Output**

{{ method.output_type.description }}

```proto
{%- if method.output_type.fields.is_empty() %}
message {{ method.output_type.name }} {}
{% else %}
message {{ method.output_type.name }} {
{%- for field in method.output_type.fields %}
  {{ field.type_name }} {{ field.name }} = {{ field.number }};
{%- endfor %}
}
{% endif -%}
```

{% endfor %}
{% endfor %}
