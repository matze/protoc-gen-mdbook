{% for service in services %}
## {{ service.name }}

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

**Input : `{{ method.input_type }}`**

**Output : `{{ method.output_type }}`**

{% endfor %}
{% endfor %}
