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

#### Input

```proto
message {{ method.input_type.name }} {}
```

#### Output

```proto
message {{ method.output_type.name }} {}
```

{% endfor %}
{% endfor %}
