"""Initial migration

Revision ID: 0e04296319d3
Revises: 
Create Date: 2020-04-06 15:47:49.534615

"""
from alembic import op
import sqlalchemy as sa


# revision identifiers, used by Alembic.
revision = "0e04296319d3"
down_revision = None
branch_labels = None
depends_on = None


def upgrade():
    op.create_table(
        "ingredient",
        sa.Column("id", sa.Integer(), autoincrement=True, nullable=False),
        sa.Column("name", sa.String(), nullable=False),
        sa.Column("slug", sa.String(), nullable=False),
        sa.Column("parent_id", sa.Integer(), nullable=True),
        sa.ForeignKeyConstraint(["parent_id"], ["ingredient.id"],),
        sa.PrimaryKeyConstraint("id"),
    )
    op.create_index(op.f("ix_ingredient_slug"), "ingredient", ["slug"], unique=True)
    op.create_table(
        "preparation",
        sa.Column("id", sa.Integer(), autoincrement=True, nullable=False),
        sa.Column("name", sa.String(), nullable=False),
        sa.PrimaryKeyConstraint("id"),
    )
    op.create_table(
        "quantity_unit",
        sa.Column("id", sa.Integer(), autoincrement=True, nullable=False),
        sa.Column("name", sa.String(), nullable=True),
        sa.Column("abbreviation", sa.String(), nullable=True),
        sa.PrimaryKeyConstraint("id"),
    )
    op.create_table(
        "recipe",
        sa.Column("id", sa.Integer(), autoincrement=True, nullable=False),
        sa.Column("title", sa.String(), nullable=False),
        sa.Column("slug", sa.String(), nullable=False),
        sa.PrimaryKeyConstraint("id"),
    )
    op.create_index(op.f("ix_recipe_slug"), "recipe", ["slug"], unique=True)
    op.create_table(
        "recipe_ingredient",
        sa.Column("ingredient_id", sa.Integer(), nullable=False),
        sa.Column("recipe_id", sa.Integer(), nullable=False),
        sa.Column("preparation_id", sa.Integer(), nullable=True),
        sa.Column("quantity_unit_id", sa.Integer(), nullable=True),
        sa.Column("quantity", sa.Numeric(), nullable=True),
        sa.ForeignKeyConstraint(["ingredient_id"], ["ingredient.id"],),
        sa.ForeignKeyConstraint(["preparation_id"], ["preparation.id"],),
        sa.ForeignKeyConstraint(["quantity_unit_id"], ["quantity_unit.id"],),
        sa.ForeignKeyConstraint(["recipe_id"], ["recipe.id"],),
        sa.PrimaryKeyConstraint("ingredient_id", "recipe_id"),
    )


def downgrade():
    op.drop_table("recipe_ingredient")
    op.drop_index(op.f("ix_recipe_slug"), table_name="recipe")
    op.drop_table("recipe")
    op.drop_table("quantity_unit")
    op.drop_table("preparation")
    op.drop_index(op.f("ix_ingredient_slug"), table_name="ingredient")
    op.drop_table("ingredient")
