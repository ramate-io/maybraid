//! Optional typed-recipe boilerplate. Layer functions receive `&self`.
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, LitStr, Path};

/// `#[effect(name = "impact", layers = Self::layers)]` calls `Self::layers(&self)`.
/// The function returns any iterator of `EffectLayer`, including `EffectLayerGroup`.
#[proc_macro_derive(EffectRecipe, attributes(effect))]
pub fn derive_effect_recipe(input: TokenStream) -> TokenStream {
	let input = parse_macro_input!(input as DeriveInput);
	expand(&input).unwrap_or_else(syn::Error::into_compile_error).into()
}

fn expand(input: &DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
	let mut name: Option<LitStr> = None;
	let mut layers: Option<Path> = None;
	for attr in input.attrs.iter().filter(|attr| attr.path().is_ident("effect")) {
		attr.parse_nested_meta(|meta| {
			if meta.path.is_ident("name") {
				if name.is_some() {
					return Err(meta.error("duplicate effect name"));
				}
				name = Some(meta.value()?.parse()?);
			} else if meta.path.is_ident("layers") {
				if layers.is_some() {
					return Err(meta.error("duplicate layer function"));
				}
				layers = Some(meta.value()?.parse()?);
			} else {
				return Err(meta.error("expected name or layers"));
			}
			Ok(())
		})?;
	}
	let name = name.ok_or_else(|| {
		syn::Error::new_spanned(input, "missing #[effect(name = \"...\", layers = path)]")
	})?;
	let layers =
		layers.ok_or_else(|| syn::Error::new_spanned(input, "missing effect layers function"))?;
	let crate_path = match proc_macro_crate::crate_name("maybraid-vfx")
		.map_err(|error| syn::Error::new(Span::call_site(), error))?
	{
		proc_macro_crate::FoundCrate::Itself => quote!(::maybraid_vfx),
		proc_macro_crate::FoundCrate::Name(name) => {
			let ident = syn::Ident::new(&name, Span::call_site());
			quote!(::#ident)
		}
	};
	let ident = &input.ident;
	let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
	Ok(quote! {
		impl #impl_generics #crate_path::EffectRecipe for #ident #ty_generics #where_clause {
			fn effect_definition(&self) -> #crate_path::EffectDefinition {
				#crate_path::EffectDefinition::new(#name, #layers(self))
			}
		}
	})
}

#[cfg(test)]
mod tests {
	use crate::expand;

	#[test]
	fn invalid_attributes_report_actionable_errors() -> syn::Result<()> {
		for (source, message) in [
			("struct Missing;", "missing #[effect"),
			("#[effect(name = \"x\")] struct MissingLayers;", "missing effect layers"),
			("#[effect(name = \"x\", name = \"y\")] struct Duplicate;", "duplicate effect name"),
			("#[effect(unknown = 2)] struct Typo;", "expected name or layers"),
		] {
			let input = syn::parse_str(source)?;
			let result = expand(&input);
			assert!(result.is_err());
			assert!(result.err().is_some_and(|error| error.to_string().contains(message)));
		}
		Ok(())
	}
}
