use crate::parser::{EnumVariantFields, TypeAnn};
use crate::types::definitions::{EnumDefinition, StructDefinition};
use crate::types::Type;
use std::collections::HashMap;

/// Memory layout information for types
#[derive(Debug, Clone)]
pub struct TypeLayout {
    /// Total size in bytes
    pub size: u32,
    /// Alignment requirement in bytes
    pub alignment: u32,
}

/// Field layout information
#[derive(Debug, Clone)]
pub struct FieldLayout {
    /// Offset from the beginning of the struct
    pub offset: u32,
    /// Size of the field
    pub size: u32,
    /// Type of the field
    pub ty: Type,
}

/// Struct memory layout
#[derive(Debug, Clone)]
pub struct StructLayout {
    /// Name of the struct
    pub name: String,
    /// Field layouts in order
    pub fields: Vec<(String, FieldLayout)>,
    /// Total size including padding
    pub total_size: u32,
    /// Alignment requirement
    pub alignment: u32,
}

/// Enum variant layout
#[derive(Debug, Clone)]
pub struct VariantLayout {
    /// Variant name
    pub name: String,
    /// Discriminant value
    pub tag: u32,
    /// Layout of variant data (if any)
    pub data_layout: VariantDataLayout,
}

/// Layout of variant data
#[derive(Debug, Clone)]
pub enum VariantDataLayout {
    Unit,
    Tuple(Vec<TypeLayout>),
    Struct(Vec<(String, FieldLayout)>),
}

/// Enum memory layout
#[derive(Debug, Clone)]
pub struct EnumLayout {
    /// Name of the enum
    pub name: String,
    /// Tag/discriminant size and offset (always at offset 0)
    pub tag_size: u32,
    /// Variant layouts
    pub variants: Vec<VariantLayout>,
    /// Maximum variant data size
    pub max_variant_size: u32,
    /// Total size (tag + max variant size + padding)
    pub total_size: u32,
    /// Alignment requirement
    pub alignment: u32,
}

/// Memory layout calculator
pub struct LayoutCalculator {
    /// Cache of calculated layouts
    struct_layouts: HashMap<String, StructLayout>,
    enum_layouts: HashMap<String, EnumLayout>,
}

impl LayoutCalculator {
    pub fn new() -> Self {
        Self {
            struct_layouts: HashMap::new(),
            enum_layouts: HashMap::new(),
        }
    }

    /// Convert TypeAnn to Type for layout calculation
    fn type_ann_to_type(&self, type_ann: &TypeAnn) -> Type {
        match &type_ann.kind {
            crate::parser::TypeKind::Named(name) => {
                // Check if it's a known type like Option or Result
                match name.as_str() {
                    "Option" => Type::Unknown, // Generic Option without args
                    "Result" => Type::Unknown, // Generic Result without args
                    _ => Type::Named(name.clone()),
                }
            }
            crate::parser::TypeKind::Array(elem) => {
                Type::Array(Box::new(self.type_ann_to_type(elem)))
            }
            crate::parser::TypeKind::Function { params, ret } => Type::Function {
                params: params.iter().map(|p| self.type_ann_to_type(p)).collect(),
                ret: Box::new(self.type_ann_to_type(ret)),
            },
            crate::parser::TypeKind::Generic { name, args } => {
                // Handle Option<T> and Result<T, E> as special cases
                match name.as_str() {
                    "Option" if args.len() == 1 => {
                        Type::Option(Box::new(self.type_ann_to_type(&args[0])))
                    }
                    "Result" if args.len() == 2 => Type::Result {
                        ok: Box::new(self.type_ann_to_type(&args[0])),
                        err: Box::new(self.type_ann_to_type(&args[1])),
                    },
                    _ => Type::Generic {
                        name: name.clone(),
                        args: args.iter().map(|a| self.type_ann_to_type(a)).collect(),
                    },
                }
            }
            crate::parser::TypeKind::TypeParam(name) => Type::TypeParam(name.clone()),
            crate::parser::TypeKind::Tuple(types) => {
                Type::Tuple(types.iter().map(|t| self.type_ann_to_type(t)).collect())
            }
            crate::parser::TypeKind::Reference { mutable, inner } => Type::Reference {
                mutable: *mutable,
                inner: Box::new(self.type_ann_to_type(inner)),
            },
        }
    }

    /// Calculate layout for a struct
    pub fn calculate_struct_layout(&mut self, def: &StructDefinition) -> StructLayout {
        // Check cache first
        if let Some(layout) = self.struct_layouts.get(&def.name) {
            return layout.clone();
        }

        let mut fields = Vec::new();
        let mut current_offset = 0u32;
        let mut max_alignment = 1u32;

        // Calculate layout for each field
        for field in &def.fields {
            let field_type = self.type_ann_to_type(&field.type_ann);
            let field_layout = self.calculate_type_layout(&field_type);
            let alignment = field_layout.alignment;

            // Align the current offset
            current_offset = align_up(current_offset, alignment);

            fields.push((
                field.name.clone(),
                FieldLayout {
                    offset: current_offset,
                    size: field_layout.size,
                    ty: field_type,
                },
            ));

            current_offset += field_layout.size;
            max_alignment = max_alignment.max(alignment);
        }

        // Align total size to struct alignment
        let total_size = align_up(current_offset, max_alignment);

        let layout = StructLayout {
            name: def.name.clone(),
            fields,
            total_size,
            alignment: max_alignment,
        };

        // Cache the result
        self.struct_layouts.insert(def.name.clone(), layout.clone());
        layout
    }

    /// Calculate layout for an enum
    pub fn calculate_enum_layout(&mut self, def: &EnumDefinition) -> EnumLayout {
        // Check cache first
        if let Some(layout) = self.enum_layouts.get(&def.name) {
            return layout.clone();
        }

        let tag_size = 4u32; // u32 discriminant
        let tag_alignment = 4u32;
        let mut max_variant_size = 0u32;
        let mut max_alignment = tag_alignment;
        let mut variants = Vec::new();

        // Calculate layout for each variant
        for (tag, variant) in def.variants.iter().enumerate() {
            let variant_layout = match &variant.fields {
                EnumVariantFields::Unit => VariantLayout {
                    name: variant.name.clone(),
                    tag: tag as u32,
                    data_layout: VariantDataLayout::Unit,
                },
                EnumVariantFields::Tuple(types) => {
                    let mut tuple_layouts = Vec::new();
                    let mut variant_size = 0u32;
                    let mut variant_alignment = 1u32;

                    for type_ann in types {
                        let ty = self.type_ann_to_type(type_ann);
                        let type_layout = self.calculate_type_layout(&ty);
                        variant_size = align_up(variant_size, type_layout.alignment);
                        variant_size += type_layout.size;
                        variant_alignment = variant_alignment.max(type_layout.alignment);
                        tuple_layouts.push(type_layout);
                    }

                    max_variant_size = max_variant_size.max(variant_size);
                    max_alignment = max_alignment.max(variant_alignment);

                    VariantLayout {
                        name: variant.name.clone(),
                        tag: tag as u32,
                        data_layout: VariantDataLayout::Tuple(tuple_layouts),
                    }
                }
                EnumVariantFields::Struct(fields) => {
                    let mut struct_fields = Vec::new();
                    let mut variant_size = 0u32;
                    let mut variant_alignment = 1u32;

                    for field in fields {
                        let field_type = self.type_ann_to_type(&field.type_ann);
                        let field_layout = self.calculate_type_layout(&field_type);
                        variant_size = align_up(variant_size, field_layout.alignment);

                        struct_fields.push((
                            field.name.clone(),
                            FieldLayout {
                                offset: variant_size,
                                size: field_layout.size,
                                ty: field_type,
                            },
                        ));

                        variant_size += field_layout.size;
                        variant_alignment = variant_alignment.max(field_layout.alignment);
                    }

                    max_variant_size = max_variant_size.max(variant_size);
                    max_alignment = max_alignment.max(variant_alignment);

                    VariantLayout {
                        name: variant.name.clone(),
                        tag: tag as u32,
                        data_layout: VariantDataLayout::Struct(struct_fields),
                    }
                }
            };

            variants.push(variant_layout);
        }

        // Calculate total size: tag + max variant size, aligned
        let data_offset = align_up(tag_size, max_alignment);
        let total_size = align_up(data_offset + max_variant_size, max_alignment);

        let layout = EnumLayout {
            name: def.name.clone(),
            tag_size,
            variants,
            max_variant_size,
            total_size,
            alignment: max_alignment,
        };

        // Cache the result
        self.enum_layouts.insert(def.name.clone(), layout.clone());
        layout
    }

    /// Calculate layout for a type
    pub fn calculate_type_layout(&self, ty: &Type) -> TypeLayout {
        match ty {
            Type::I32 => TypeLayout {
                size: 4,
                alignment: 4,
            },
            Type::F32 => TypeLayout {
                size: 4,
                alignment: 4,
            },
            Type::Bool => TypeLayout {
                size: 1,
                alignment: 1,
            },
            Type::String => TypeLayout {
                size: 16,
                alignment: 8,
            }, // Fat pointer: ptr + len
            Type::Array(_) => TypeLayout {
                size: 16,
                alignment: 8,
            }, // Fat pointer: ptr + len
            Type::Named(name) => {
                // Check if it's a known struct or enum
                if let Some(struct_layout) = self.struct_layouts.get(name) {
                    TypeLayout {
                        size: struct_layout.total_size,
                        alignment: struct_layout.alignment,
                    }
                } else if let Some(enum_layout) = self.enum_layouts.get(name) {
                    TypeLayout {
                        size: enum_layout.total_size,
                        alignment: enum_layout.alignment,
                    }
                } else {
                    // Default for unknown named types
                    TypeLayout {
                        size: 8,
                        alignment: 8,
                    }
                }
            }
            Type::Generic { name, .. } => {
                // For generic types, check if it's a monomorphized version
                if let Some(struct_layout) = self.struct_layouts.get(name) {
                    TypeLayout {
                        size: struct_layout.total_size,
                        alignment: struct_layout.alignment,
                    }
                } else if let Some(enum_layout) = self.enum_layouts.get(name) {
                    TypeLayout {
                        size: enum_layout.total_size,
                        alignment: enum_layout.alignment,
                    }
                } else {
                    // Default for unknown generic types
                    TypeLayout {
                        size: 8,
                        alignment: 8,
                    }
                }
            }
            Type::Function { .. } => TypeLayout {
                size: 8,
                alignment: 8,
            }, // Function pointer
            Type::Tuple(types) => {
                let mut size = 0u32;
                let mut alignment = 1u32;

                for ty in types {
                    let layout = self.calculate_type_layout(ty);
                    size = align_up(size, layout.alignment);
                    size += layout.size;
                    alignment = alignment.max(layout.alignment);
                }

                TypeLayout {
                    size: align_up(size, alignment),
                    alignment,
                }
            }
            Type::Reference { .. } => TypeLayout {
                size: 8,
                alignment: 8,
            }, // Pointer
            Type::Unknown | Type::Never => TypeLayout {
                size: 0,
                alignment: 1,
            },
            Type::TypeVar(_) | Type::TypeParam(_) => {
                // Type variables should be resolved before layout calculation
                TypeLayout {
                    size: 8,
                    alignment: 8,
                }
            }
            Type::Option(_) | Type::Result { .. } | Type::Future(_) => {
                // These would need special handling based on their representation
                TypeLayout {
                    size: 16,
                    alignment: 8,
                } // Conservative estimate
            }
            Type::Struct { name, .. } => {
                // Look up the struct layout or calculate a default
                if let Some(struct_layout) = self.struct_layouts.get(name) {
                    TypeLayout {
                        size: struct_layout.total_size,
                        alignment: struct_layout.alignment,
                    }
                } else {
                    TypeLayout {
                        size: 16,
                        alignment: 8,
                    }
                }
            }
        }
    }

    /// Get the offset of a struct field
    pub fn get_field_offset(&self, struct_name: &str, field_name: &str) -> Option<u32> {
        self.struct_layouts.get(struct_name).and_then(|layout| {
            layout
                .fields
                .iter()
                .find(|(name, _)| name == field_name)
                .map(|(_, field)| field.offset)
        })
    }

    /// Get the tag value for an enum variant
    pub fn get_variant_tag(&self, enum_name: &str, variant_name: &str) -> Option<u32> {
        self.enum_layouts.get(enum_name).and_then(|layout| {
            layout
                .variants
                .iter()
                .find(|v| v.name == variant_name)
                .map(|v| v.tag)
        })
    }

    /// Get the enum layout
    pub fn get_enum_layout(&self, enum_name: &str) -> Option<&EnumLayout> {
        self.enum_layouts.get(enum_name)
    }

    /// Get variant layout information
    pub fn get_variant_layout(
        &self,
        enum_name: &str,
        variant_name: &str,
    ) -> Option<&VariantLayout> {
        self.enum_layouts
            .get(enum_name)
            .and_then(|layout| layout.variants.iter().find(|v| v.name == variant_name))
    }
}

/// Align a value up to the given alignment
fn align_up(value: u32, alignment: u32) -> u32 {
    (value + alignment - 1) & !(alignment - 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::{StructField, TypeAnn};
    use crate::source::{SourceLocation, Span};

    fn dummy_span() -> Span {
        Span::new(SourceLocation::new(1, 1, 0), SourceLocation::new(1, 1, 0))
    }

    #[test]
    fn test_primitive_layouts() {
        let calc = LayoutCalculator::new();

        let i32_layout = calc.calculate_type_layout(&Type::I32);
        assert_eq!(i32_layout.size, 4);
        assert_eq!(i32_layout.alignment, 4);

        let bool_layout = calc.calculate_type_layout(&Type::Bool);
        assert_eq!(bool_layout.size, 1);
        assert_eq!(bool_layout.alignment, 1);
    }

    #[test]
    fn test_struct_layout() {
        let mut calc = LayoutCalculator::new();

        let def = StructDefinition {
            name: "Point".to_string(),
            generic_params: None,
            fields: vec![
                StructField {
                    name: "x".to_string(),
                    type_ann: TypeAnn {
                        kind: crate::parser::TypeKind::Named("i32".to_string()),
                        span: dummy_span(),
                    },
                    span: dummy_span(),
                },
                StructField {
                    name: "y".to_string(),
                    type_ann: TypeAnn {
                        kind: crate::parser::TypeKind::Named("i32".to_string()),
                        span: dummy_span(),
                    },
                    span: dummy_span(),
                },
            ],
            where_clause: None,
            span: dummy_span(),
            is_monomorphized: false,
            original_type: None,
        };

        let layout = calc.calculate_struct_layout(&def);
        assert_eq!(layout.total_size, 8); // 4 + 4
        assert_eq!(layout.alignment, 4);
        assert_eq!(layout.fields.len(), 2);
        assert_eq!(layout.fields[0].1.offset, 0);
        assert_eq!(layout.fields[1].1.offset, 4);
    }

    #[test]
    fn test_align_up() {
        assert_eq!(align_up(0, 4), 0);
        assert_eq!(align_up(1, 4), 4);
        assert_eq!(align_up(4, 4), 4);
        assert_eq!(align_up(5, 4), 8);
        assert_eq!(align_up(7, 8), 8);
        assert_eq!(align_up(8, 8), 8);
    }
}
