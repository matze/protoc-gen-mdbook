{%- if fields.is_empty() && nested.is_empty() %}
{{ depth|lead }}message {{ name }} {}
{% else %}
{{ depth|lead }}message {{ name }} {
{%- for message_type in nested -%}
{{ message_type.render().unwrap() }}
{%- endfor -%}
{%- for field in fields -%}
  {% if field.leading_comments != "" -%}
  {{ depth|lead }}{{ field.leading_comments|render_multiline_comment|indent(2) }}
  {%- endif %}
  {{ depth|lead }}{% if field.optional %}optional {% endif %}{% if field.repeated %}repeated {% endif %}{{ field.ty.name() }} {{ field.name }} = {{ field.number }}; {% if field.trailing_comments != "" %} // {{- field.trailing_comments -}}
  {% endif %}
{%- endfor %}
{{ depth|lead }}}
{% endif -%}
