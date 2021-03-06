"""initial migration

Revision ID: 46b1bd044f5f
Revises: 
Create Date: 2020-04-27 13:50:50.270943

"""
from alembic import op
import sqlalchemy as sa


# revision identifiers, used by Alembic.
revision = "46b1bd044f5f"
down_revision = None
branch_labels = None
depends_on = None


def upgrade():
    # ### commands auto generated by Alembic - please adjust! ###
    op.create_table(
        "ingredients",
        sa.Column("slug", sa.String(), nullable=False),
        sa.Column("url", sa.String(), nullable=False),
        sa.Column("created_at", sa.DateTime(), nullable=True),
        sa.Column("updated_at", sa.DateTime(), nullable=True),
        sa.Column("id", sa.Integer(), autoincrement=True, nullable=False),
        sa.Column("name", sa.String(), nullable=False),
        sa.Column("parent_id", sa.Integer(), nullable=True),
        sa.ForeignKeyConstraint(["parent_id"], ["ingredients.id"],),
        sa.PrimaryKeyConstraint("id"),
    )
    op.create_index(op.f("ix_ingredients_slug"), "ingredients", ["slug"], unique=True)
    op.create_index(op.f("ix_ingredients_url"), "ingredients", ["url"], unique=True)
    op.create_table(
        "quantity_units",
        sa.Column("created_at", sa.DateTime(), nullable=True),
        sa.Column("updated_at", sa.DateTime(), nullable=True),
        sa.Column("id", sa.Integer(), autoincrement=True, nullable=False),
        sa.Column("name", sa.String(), nullable=True),
        sa.Column("abbreviation", sa.String(), nullable=True),
        sa.PrimaryKeyConstraint("id"),
    )
    op.create_table(
        "recipes",
        sa.Column("slug", sa.String(), nullable=False),
        sa.Column("url", sa.String(), nullable=False),
        sa.Column("created_at", sa.DateTime(), nullable=True),
        sa.Column("updated_at", sa.DateTime(), nullable=True),
        sa.Column("id", sa.Integer(), autoincrement=True, nullable=False),
        sa.Column("title", sa.String(), nullable=False),
        sa.Column("chef_name", sa.String(), nullable=True),
        sa.Column("image_name", sa.String(), nullable=True),
        sa.PrimaryKeyConstraint("id"),
    )
    op.create_index(op.f("ix_recipes_slug"), "recipes", ["slug"], unique=True)
    op.create_index(op.f("ix_recipes_url"), "recipes", ["url"], unique=True)
    op.create_table(
        "recipe_ingredients",
        sa.Column("created_at", sa.DateTime(), nullable=True),
        sa.Column("updated_at", sa.DateTime(), nullable=True),
        sa.Column("id", sa.Integer(), autoincrement=True, nullable=False),
        sa.Column("recipe_id", sa.Integer(), nullable=False),
        sa.Column("ingredient_id", sa.Integer(), nullable=True),
        sa.Column("quantity_unit_id", sa.Integer(), nullable=True),
        sa.Column("quantity", sa.Numeric(), nullable=True),
        sa.Column("description", sa.String(), nullable=False),
        sa.ForeignKeyConstraint(["ingredient_id"], ["ingredients.id"],),
        sa.ForeignKeyConstraint(["quantity_unit_id"], ["quantity_units.id"],),
        sa.ForeignKeyConstraint(["recipe_id"], ["recipes.id"],),
        sa.PrimaryKeyConstraint("id"),
    )
    # ### end Alembic commands ###


def downgrade():
    # ### commands auto generated by Alembic - please adjust! ###
    op.drop_table("recipe_ingredients")
    op.drop_index(op.f("ix_recipes_url"), table_name="recipes")
    op.drop_index(op.f("ix_recipes_slug"), table_name="recipes")
    op.drop_table("recipes")
    op.drop_table("quantity_units")
    op.drop_index(op.f("ix_ingredients_url"), table_name="ingredients")
    op.drop_index(op.f("ix_ingredients_slug"), table_name="ingredients")
    op.drop_table("ingredients")
    # ### end Alembic commands ###
