from transformers import AutoModel, AutoTokenizer


def download_embedding_model(model_name, output_file):
    """Downloads a text embedding model and stores it to a file.

    Args:
        model_name: The name of the model from the Hugging Face model hub (e.g., "sentence-transformers/all-mpnet-base-v2").
        output_file: The path to the file where the model will be stored.
    """
    # Download model and tokenizer
    model = AutoModel.from_pretrained(model_name)
    tokenizer = AutoTokenizer.from_pretrained(model_name)

    # Save model and tokenizer
    model.save_pretrained(output_file)
    tokenizer.save_pretrained(output_file)

    print(f"Downloaded model and tokenizer to: {output_file}")


# Example usage
model_name = "sentence-transformers/all-mpnet-base-v2"
output_file = "my_text_embedding_model"

download_embedding_model(model_name, output_file)
